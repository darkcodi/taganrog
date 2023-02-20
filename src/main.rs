use anyhow::Context;
use clap::Parser;
use surrealdb::Datastore;
use tracing::info;
use taganrog::config::{Config, FlatConfig};
use taganrog::http;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // env_logger::init();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config: Config = FlatConfig::parse().into();

    info!("Connecting to DB...");
    let db = Datastore::new(&config.db.database_url)
        .await
        .context("Database connection failed")?;
    info!("Connected to DB!");

    http::serve(config, db).await?;

    Ok(())
}
