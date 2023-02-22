use clap::Parser;
use taganrog::config::{Config, FlatConfig};
use taganrog::{db, http};
use taganrog::http::ApiContext;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config: Config = FlatConfig::parse().into();
    let ctx = ApiContext::new(config);

    db::migrate(ctx.clone()).await?;

    http::serve(ctx).await?;

    Ok(())
}
