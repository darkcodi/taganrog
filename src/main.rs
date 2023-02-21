use clap::Parser;
use taganrog::config::{Config, FlatConfig};
use taganrog::http;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config: Config = FlatConfig::parse().into();

    http::serve(config).await?;

    Ok(())
}
