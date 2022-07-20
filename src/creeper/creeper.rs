use anyhow::{anyhow, Context};
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use nonzero_ext::*;
use quick_xml::de::from_str;
use reqwest::header::HeaderMap;
use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;
use serenity::http::Http;
use std::num::NonZeroU32;

use crate::creeper::User;

use crate::creeper::book::Book;
use crate::discord::post_book;
use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};

pub async fn creep(http: impl AsRef<Http>, pool: SqlitePool) -> anyhow::Result<()> {
    let client = GovernedClient::default();
    loop {
        if let Some(users) = User::get_refreshable_users(&pool, 5).await? {
            for i in 0..users.len() {
                let books = dbg!(check_rss(&users[i], &pool, &client).await);
                if let Some(books) = books {
                    for j in 0..books.len() {
                        post_book(&http, &books[j], &users[i])
                            .await
                            .expect("Unable to post book to discord!");
                    }
                }
            }
        }
        sleep(Duration::from_millis(1000 * 60)).await;
    }
}

async fn check_rss(user: &User, pool: &SqlitePool, client: &GovernedClient) -> Option<Vec<Book>> {
    let url = format!(
        "https://www.goodreads.com/review/list_rss/{}?shelf=read",
        user.goodreads_user_id
    );

    let response = pull_rss(&client, &url, &user.last_etag).await;

    match response {
        Ok(RssResult { rss, etag }) => {
            if let Some(last_book_id) = &user.last_book_id {
                // get items up to the last book id
                let mut book_list: Vec<Book> = Vec::new();
                for i in 0..rss.channel.items.len() {
                    let item = &rss.channel.items[i];
                    if &item.id != last_book_id {
                        if let Ok(book) = item.try_into() {
                            book_list.push(book);
                        }
                    } else {
                        break;
                    }
                }
                if !book_list.is_empty() {
                    let first = book_list.first().expect("no items in booklist");
                    user.update(pool, first.id(), etag)
                        .await
                        .expect("unable to update database");
                    Some(book_list)
                } else {
                    user.update_timestamp(pool)
                        .await
                        .expect("unable to update timestamp in database");
                    None
                }
            } else {
                // never run
                let item = rss.channel.items.first().expect("No items in the channel!");
                let last_isbn = item.id.to_string();
                user.update(pool, &last_isbn, etag)
                    .await
                    .expect("unable to update database");
                None
            }
        }
        Err(why) => {
            println!("RSS Pull failed because: {}", why);
            user.update_timestamp(pool)
                .await
                .expect("unable to update timestamp in database");
            None
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Default)]
struct Item {
    #[serde(rename = "pubDate", default)]
    pub_date: String,
    #[serde(rename = "book_id", default)]
    id: String,
    user_rating: usize,
    title: String,
    link: String,
    #[serde(rename = "author_name", default)]
    author: String,
    #[serde(rename = "book_medium_image_url", default)]
    image_url: String,
}

impl TryInto<Book> for &Item {
    type Error = String;

    fn try_into(self) -> Result<Book, Self::Error> {
        Book::new(
            &self.title,
            &self.link,
            &self.pub_date,
            &self.id,
            self.user_rating,
            &self.author,
            &self.image_url,
        )
        .ok_or("Unable to create Book from Item".to_string())
    }
}

#[derive(Debug, Deserialize, PartialEq, Default)]
struct Channel {
    title: String,
    #[serde(rename = "item", default)]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Rss {
    #[serde(rename = "channel", default)]
    channel: Channel,
}

struct RssResult {
    rss: Rss,
    etag: Option<String>,
}

async fn pull_rss(
    client: &GovernedClient,
    url: &str,
    last_etag: &Option<String>,
) -> anyhow::Result<RssResult> {
    let resp = if let Some(last_etag) = last_etag {
        client.get_if_etag_modified(url, last_etag).await?
    } else {
        Some(client.get(url).await?)
    }
    .ok_or(anyhow!("ETAG is unmodified"))?;

    if resp.status().as_u16() != 200 {
        return Err(anyhow!(
            "GET request returned HTTP {}",
            resp.status().as_u16()
        ));
    }
    let etag = resp
        .headers()
        .get("etag")
        .map(|etag| etag.to_str().map(|s| s.to_owned()))
        .transpose()
        .unwrap_or(None);

    let content = resp
        .text()
        .await
        .with_context(|| "Unable to parse response to bytes")?;

    let rss: Rss = from_str(&content).expect("Unable deserialize string");

    Ok(RssResult { rss, etag })
}

struct GovernedClient {
    client: Client,
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>,
}

impl Default for GovernedClient {
    fn default() -> Self {
        GovernedClient::new(Client::default(), nonzero!(1u32))
    }
}
impl GovernedClient {
    fn new(client: Client, limit_per_second: NonZeroU32) -> Self {
        Self {
            client,
            limiter: RateLimiter::direct(Quota::per_second(limit_per_second)),
        }
    }

    async fn get(&self, url: &str) -> anyhow::Result<Response> {
        self.limiter.until_ready().await;
        self.client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Unable to get url: {}", url))
    }

    async fn head(&self, url: &str, headers: Option<HeaderMap>) -> anyhow::Result<Response> {
        self.limiter.until_ready().await;
        headers
            .map_or_else(
                || self.client.head(url).send(),
                |headers| self.client.head(url).headers(headers).send(),
            )
            .await
            .with_context(|| format!("Unable to get url: {}", url))
    }

    async fn get_if_etag_modified(
        &self,
        url: &str,
        etag: &str,
    ) -> anyhow::Result<Option<Response>> {
        let mut header_map = HeaderMap::new();
        header_map.append(
            "If-None-Match",
            etag.parse()
                .with_context(|| format!("Unable to parse etag: {}", etag))?,
        );
        let etag_check = self
            .head(url, Some(header_map))
            .await
            .with_context(|| "unable to to HTTP HEAD the rss feed")?;

        if etag_check.status() == StatusCode::NOT_MODIFIED {
            return Ok(None);
        }
        let result = self
            .get(url)
            .await
            .with_context(|| format!("Unable to get url: {}", url))?;
        Ok(Some(result))
    }
}
