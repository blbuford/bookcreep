use anyhow::{anyhow, Context};
use quick_xml::de::from_str;
use serenity::http::Http;
use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};

use crate::crawler::{GovernedClient, Rss, RssResult};
use crate::discord::post_book;
use crate::model::Book;
use crate::model::User;

pub async fn crawl(http: impl AsRef<Http>, pool: SqlitePool) -> anyhow::Result<()> {
    let client = GovernedClient::default();
    loop {
        if let Some(users) = User::get_refreshable_users(&pool, 5).await? {
            for i in 0..users.len() {
                let books = check_rss(&users[i], &pool, &client).await;
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

pub async fn pull_rss(
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
