use std::env;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use std::sync::Arc;

use crate::crawler::crawl;
use crate::discord::get_discord_client;

pub async fn run_until_stopped() -> anyhow::Result<()> {
    let database = Arc::new(
        sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::from_str(&env::var("DATABASE_URL")?)?
                    .create_if_missing(true),
            )
            .await
            .expect("Couldn't connect to database"),
    );

    loop {
        let mut discord_client = get_discord_client(database.clone()).await;
        let cache_and_http = discord_client.cache_and_http.clone();
        tokio::select! {
            r = discord_client.start() => { report_exit("Discord Client", r) },
            r = crawl(cache_and_http, database.clone()) => { report_exit("Crawler", r)},
        };
    }
}

fn report_exit(task_name: &str, outcome: Result<(), impl Debug + Display>) {
    match outcome {
        Ok(()) => {
            tracing::info!("{} has exited", task_name)
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed to complete",
                task_name
            )
        }
    }
}
