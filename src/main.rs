use anyhow::Context;
use clap::Parser;
use sea_orm::Database;
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

    info!("Connecting pool to DB...");
    let db = Database::connect(&config.db.database_url)
        .await
        .context("Database connection failed")?;
    info!("Connected to DB!");

    // Migrator::up(&conn, None).await.unwrap();

    http::serve(config, db).await?;

    Ok(())
}
