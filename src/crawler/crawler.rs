use anyhow::{anyhow, Context};
use quick_xml::de::from_str;
use serenity::http::Http;
use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};

use crate::crawler::{GovernedClient, Rss, RssResult};
use crate::discord::post_book;
use crate::model::Book;
use crate::model::User;

pub async fn crawl(http: impl AsRef<Http>, pool: &SqlitePool) -> anyhow::Result<()> {
    let client = GovernedClient::default();

    loop {
        if let Some(mut users) = User::get_refreshable_users(&pool, 5).await? {
            for mut user in users.iter_mut() {
                match check_rss(&mut user, &client).await {
                    Ok(result) => {
                        if let Some(books) = result {
                            for book in books.iter() {
                                post_book(&http, &book, &user)
                                    .await
                                    .context("Unable to post book to discord!")?;
                            }
                        }
                        user.update(pool)
                            .await
                            .context("unable to update timestamp in database")?;
                    }
                    Err(why) => {
                        tracing::error!(
                            error.cause_chain = ?why,
                            error.message = %why,
                            "RSS check failed because: {}",
                            why
                        );
                    }
                }
            }
        }
        sleep(Duration::from_millis(1000 * 60)).await;
    }
}
#[tracing::instrument(name = "Checking a user's RSS feed", skip(client))]
async fn check_rss(user: &mut User, client: &GovernedClient) -> anyhow::Result<Option<Vec<Book>>> {
    let url = format!(
        "https://www.goodreads.com/review/list_rss/{}?shelf=read",
        user.goodreads_user_id
    );

    let RssResult { rss, etag } = get_rss_feed(&client, &url, &user.last_etag).await?;
    user.set_last_etag(etag);
    if let Some(last_book_id) = &user.last_book_id {
        // get items up to the last book id
        let mut book_list: Vec<Book> = Vec::new();
        for item in rss.channel.items.iter() {
            if &item.id != last_book_id {
                if let Ok(book) = item.try_into() {
                    book_list.push(book);
                }
            } else {
                break;
            }
        }
        if !book_list.is_empty() {
            return Ok(Some(book_list));
        }
    } else {
        // Crawler has never run for this user
        if let Some(item) = dbg!(rss.channel.items.first()) {
            user.set_last_book_id(Some(item.id.to_string()));
        }
    }

    Ok(None)
}

#[tracing::instrument(name = "Retrieving RSS feed", skip(client))]
pub async fn get_rss_feed(
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
        .with_context(|| "Unable to get text from response")?;

    let rss: Rss = from_str(&content).with_context(|| "Unable deserialize response")?;

    Ok(RssResult { rss, etag })
}
