use std::env;

use sqlx::sqlite::SqlitePool;

use bookcreep::creeper::creep;
use bookcreep::discord::get_discord_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;

    let mut discord_client = get_discord_client().await;
    let cache_and_http = discord_client.cache_and_http.clone();

    tokio::join!(
        discord_client.start(),
        creep(&cache_and_http.http, 996656225871740971, pool)
    );
    Ok(())
}
