use bookcreep::startup::run_until_stopped;
use bookcreep::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("bookcreep".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    run_until_stopped().await?;

    Ok(())
}
