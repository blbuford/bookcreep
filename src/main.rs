use std::env;
use std::str::FromStr;

use bookcreep::crawler::crawl;
use bookcreep::discord::get_discord_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(&env::var("DATABASE_URL")?)?
                .create_if_missing(true),
        )
        .await
        .expect("Couldn't connect to database");

    // Run migrations, which updates the database's schema to the latest version.
    // sqlx::migrate!("./migrations")
    //     .run(&database)
    //     .await
    //     .expect("Couldn't run database migrations");

    let mut discord_client = get_discord_client().await;
    let cache_and_http = discord_client.cache_and_http.clone();
    let _channel_no: i64 = 996656225871740971;
    let (discord_result, creeper_result) = tokio::join!(
        discord_client.start(),
        crawl(&cache_and_http.http, database)
    );

    if let Err(why) = discord_result {
        println!("Discord client failed because {}", why)
    }
    if let Err(why) = creeper_result {
        println!("Creeper client failed because {}", why)
    }
    Ok(())
}
