use clap::Parser;
use path_absolutize::Absolutize;
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

    let mut config: Config = Config::parse();
    let workdir = std::path::Path::new(&config.workdir).absolutize_from(std::env::current_dir()?)?;
    if !workdir.exists() {
        std::fs::create_dir_all(&workdir)?;
    }
    config.workdir = workdir.to_str().unwrap().to_string();
    info!("{:?}", &config);

    let db_path = std::path::Path::new(&config.workdir).join("taganrog.db");
    let db_conn = Connection::open(db_path).await.expect("failed to open db connection");

    let ctx = ApiContext::new(config, db_conn);
    db::migrate(ctx.clone()).await?;
    http::serve(ctx).await?;

    Ok(())
}
