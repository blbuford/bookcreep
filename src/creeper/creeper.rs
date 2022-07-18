use reqwest::{Client, StatusCode};
use rss::Channel;
use serenity::{http::Http, model::id::ChannelId};

use crate::creeper::User;

use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};
use crate::creeper::book::Book;

pub async fn creep(
    http: impl AsRef<Http>,
    channel_id: u64,
    pool: SqlitePool,
) -> anyhow::Result<()> {
    let channel_id = ChannelId(channel_id);
    loop {
        //channel_id.say(&http, "boopsickle").await?;
        if let Some(users) = User::get_refreshable_users(&pool, 5).await? {
            for i in 0..users.len(){
                check_rss(&users[i]).await;
            }
        } else {
            sleep(Duration::from_millis(1000 * 60 * 5)).await;
        }
    }
}

async fn check_rss(user: &User) -> Option<String> {
    let client = reqwest::ClientBuilder::new()
        .build()
        .expect("Failed to build a reqwest client.");
    let url = format!("https://www.goodreads.com/review/list_rss/{}?shelf=read", user.goodreads_user_id);
    let rss_channel = match &user.last_etag {
        None => Some(pull_rss(&client, &url).await),
        Some(etag) => {
            // check etag, if necessary do a fresh pull
            let etag_check = client.head(&url)
                .header("If-None-Match", etag)
                .send()
                .await
                .expect("unable to to HTTP HEAD the rss feed");
            if etag_check.status() == StatusCode::NOT_MODIFIED {
                None
            } else {
                Some(pull_rss(&client, &url).await)
            }
        }
    };
    if let Some(ch) = rss_channel {
        if let Some(last_isbn) = &user.last_isbn {
            // get items up to the last isbn
        } else {
            // never run
            let item = ch.items().first().expect("No items in the channel!");
            let book = Book::new(
                item.title().unwrap(),
                item.link().unwrap(),
                item.pub_date().unwrap());
            dbg!(item);

        }

    }

    None
}

async fn pull_rss(client: &Client, url: &str) -> Channel {
    let content = client.get(url)
        .send()
        .await
        .expect(&*format!("Unable to get url: {}", url))
        .bytes()
        .await
        .expect("Unable to parse response to bytes");
    let channel = Channel::read_from(&content[..]).expect("unable to create channel from bytes");
    channel
}
