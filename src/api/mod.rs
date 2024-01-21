use std::path::PathBuf;
use axum::Extension;
use std::sync::Arc;
use jammdb::DB;
use path_absolutize::Absolutize;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
pub use error::{ApiError};
use crate::db::DbRepo;
use crate::api::controllers::{media, ping, tags};

mod error;
mod controllers;

pub const CONTENT_TYPE_HEADER: &str = "content-type";
pub const APPLICATION_JSON: &str = "application/json";

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

pub async fn serve(workdir: &str) {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let workdir = get_or_create_workdir_path(workdir).expect("failed to get or create workdir path");
    let db_path = get_or_create_db_path(&workdir).expect("failed to get or create db path");
    let config: ApiConfig = ApiConfig { workdir };
    info!("{:?}", &config);

    let db = DB::open(&db_path).expect("failed to open db connection");
    let db_repo = DbRepo::new(db);

    let ctx = ApiContext::new(config, db_repo);

    let app = ping::router()
        .merge(media::router())
        .merge(tags::router())
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
    Ok(workdir)
}

fn get_or_create_db_path(workdir: &PathBuf) -> anyhow::Result<PathBuf> {
    let db_path = workdir.join("taganrog.db");
    if db_path.exists() && !db_path.is_file() {
        anyhow::bail!("db_path is not a file");
    }
    Ok(db_path)
}

#[derive(Debug)]
pub struct ApiConfig {
    pub workdir: PathBuf,
}

#[derive(Clone)]
pub struct ApiContext {
    pub cfg: Arc<ApiConfig>,
    pub db: DbRepo,
}

impl ApiContext {
    pub fn new(config: ApiConfig, db: DbRepo) -> Self {
        Self {
            cfg: Arc::new(config),
            db,
        }
    }
}
