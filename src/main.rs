use anyhow::Context;
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
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
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.db.database_url)
        .await
        .context("could not connect to database")?;
    info!("Connected to DB!");

    // sqlx::migrate!().run(&db).await?;

    http::serve(config, db).await?;

    Ok(())
}
