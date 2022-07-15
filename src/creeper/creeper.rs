use serenity::{http::Http, model::id::ChannelId};

use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};

pub async fn creep(http: impl AsRef<Http>, channel_id: u64, pool: SqlitePool) {
    let channel_id = ChannelId(channel_id);
    loop {
        channel_id.say(&http, "boopsickle").await;
        sleep(Duration::from_millis(1000 * 10)).await;
    }
}