use tokio_rusqlite::Connection;
use tracing::info;
use taganrog_d::config::Config;
use taganrog_d::{db, http};
use taganrog_d::http::ApiContext;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config: Config = Config::parse().expect("failed to parse config");
    info!("{:?}", &config);

    let db_conn = Connection::open(&config.db_path).await.expect("failed to open db connection");

    let ctx = ApiContext::new(config, db_conn);
    db::migrate(ctx.clone()).await?;
    http::serve(ctx).await?;

    Ok(())
}
