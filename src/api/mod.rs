use std::path::PathBuf;
use axum::Extension;
use std::sync::Arc;
use path_absolutize::Absolutize;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
pub use error::{ApiError};
use crate::db;
use crate::db::WalDb;

mod error;
mod controllers;
pub mod client;

pub const CONTENT_TYPE_HEADER: &str = "content-type";
pub const APPLICATION_JSON: &str = "application/json";

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

pub async fn serve(workdir: &str) {
    let tracing_layer = tracing_subscriber::fmt::layer();
    let filter = filter::Targets::new()
        // .with_target("tower_http::trace::on_request", Level::DEBUG)
        .with_target("tower_http::trace::on_response", Level::DEBUG)
        .with_target("tower_http::trace::make_span", Level::DEBUG)
        // .with_target("taganrog", Level::DEBUG)
        .with_default(Level::INFO);
    tracing_subscriber::registry()
        .with(tracing_layer)
        .with(filter)
        .init();

    let workdir = get_or_create_workdir_path(workdir).expect("failed to get or create workdir path");
    let db_path = get_or_create_db_path(&workdir).expect("failed to get or create db path");
    let config: ApiConfig = ApiConfig { workdir, db_path };
    info!("{:?}", &config);

    let db = WalDb::new(config.db_path.clone());

    let ctx = ApiContext {
        cfg: Arc::new(config),
        db: Arc::new(RwLock::new(db)),
    };
    db::init(&ctx).await.expect("failed to init db");

    let app = controllers::router()
        .layer(CorsLayer::new().allow_methods(Any).allow_headers(Any).allow_origin(Any))
        .layer(ServiceBuilder::new().layer(Extension(ctx)).layer(TraceLayer::new_for_http()),
    );

    let addr = "[::]:1698";
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind to address");
    info!("listening on {}", &addr);
    axum::serve(listener, app).await.expect("error running HTTP server");
}

fn get_or_create_workdir_path(workdir: &str) -> anyhow::Result<PathBuf> {
    info!("workdir: {}", workdir);
    let workdir = std::path::Path::new(workdir).absolutize_from(std::env::current_dir()?)?;
    if !workdir.exists() {
        std::fs::create_dir_all(&workdir)?;
    }
    if !workdir.is_dir() {
        anyhow::bail!("workdir is not a directory");
    }
    let workdir = workdir.canonicalize()?;
    Ok(workdir)
}

fn get_or_create_db_path(workdir: &PathBuf) -> anyhow::Result<PathBuf> {
    info!("db_path: {}", workdir.display());
    let db_path = workdir.join("taganrog.db.json");
    if db_path.exists() && !db_path.is_file() {
        anyhow::bail!("db_path is not a file");
    }
    if !db_path.exists() {
        std::fs::write(&db_path, "")?;
    }
    Ok(db_path)
}

#[derive(Debug)]
pub struct ApiConfig {
    pub workdir: PathBuf,
    pub db_path: PathBuf,
}

#[derive(Clone)]
pub struct ApiContext {
    pub cfg: Arc<ApiConfig>,
    pub db: Arc<RwLock<WalDb>>,
}
