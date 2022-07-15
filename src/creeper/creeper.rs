use serenity::{http::Http, model::id::ChannelId};

use crate::creeper::User;

use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};

pub async fn creep(
    http: impl AsRef<Http>,
    channel_id: u64,
    pool: SqlitePool,
) -> anyhow::Result<()> {
    let channel_id = ChannelId(channel_id);
    loop {
        channel_id.say(&http, "boopsickle").await?;
        if let Some(users) = User::get_refreshable_users(&pool, 5).await? {
            users.iter().for_each(|user| {
                // call rss checker with the user obj.
            })
        } else {
            sleep(Duration::from_millis(1000 * 60 * 5)).await;
        }
    }
}

async fn check_rss(user: User) -> Option<String> {
    
}
