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
    let thumbnails_dir = get_or_create_thumbnails_dir(&workdir).expect("failed to get or create thumbnails dir");
    let config: ApiConfig = ApiConfig { workdir, db_path, thumbnails_dir };
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
    let workdir = std::path::Path::new(workdir).absolutize_from(std::env::current_dir()?)?;
    if !workdir.exists() {
        std::fs::create_dir_all(&workdir)?;
    }
    if !workdir.is_dir() {
        anyhow::bail!("workdir is not a directory");
    }
    let workdir = workdir.canonicalize()?;
    info!("workdir: {}", workdir.display());
    Ok(workdir)
}

fn get_or_create_db_path(workdir: &PathBuf) -> anyhow::Result<PathBuf> {
    let db_path = workdir.join("taganrog.db.json");
    if !db_path.exists() {
        std::fs::write(&db_path, "")?;
    }
    if db_path.exists() && !db_path.is_file() {
        anyhow::bail!("db_path is not a file");
    }
    info!("db_path: {}", workdir.display());
    Ok(db_path)
}

fn get_or_create_thumbnails_dir(workdir: &PathBuf) -> anyhow::Result<PathBuf> {
    let thumbnails_dir = workdir.join("taganrog-thumbnails");
    if !thumbnails_dir.exists() {
        std::fs::create_dir_all(&thumbnails_dir)?;
    }
    if thumbnails_dir.exists() && !thumbnails_dir.is_dir() {
        anyhow::bail!("thumbnails_dir is not a directory");
    }
    info!("thumbnails_dir: {}", thumbnails_dir.display());
    Ok(thumbnails_dir)
}

#[derive(Debug)]
pub struct ApiConfig {
    pub workdir: PathBuf,
    pub db_path: PathBuf,
    pub thumbnails_dir: PathBuf,
}

#[derive(Clone)]
pub struct ApiContext {
    pub cfg: Arc<ApiConfig>,
    pub db: Arc<RwLock<WalDb>>,
}
