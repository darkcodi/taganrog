use jammdb::DB;
use tracing::info;
use taganrog_d::config::Config;
use taganrog_d::{db, http};
use taganrog_d::http::ApiContext;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config: Config = Config::parse().expect("failed to parse config");
    info!("{:?}", &config);

    let db = DB::open(&config.db_path).expect("failed to open db connection");
    let db_repo = db::DbRepo::new(db);

    let ctx = ApiContext::new(config, db_repo);
    http::serve(ctx).await;
}
