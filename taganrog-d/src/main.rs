use tokio_rusqlite::Connection;
use tracing::info;
use taganrog_d::config::Config;
use taganrog_d::{db, http};
use taganrog_d::http::ApiContext;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config: Config = Config::parse().expect("failed to parse config");
    info!("Config: {:?}", &config);

    let db_path = config.db_path.to_str().expect("failed to convert db path to str");
    let db_conn = Connection::open(db_path).await.expect("failed to open db connection");

    let ctx = ApiContext::new(config, db_conn);
    db::migrate(ctx.clone()).await?;
    http::serve(ctx).await?;

    Ok(())
}
