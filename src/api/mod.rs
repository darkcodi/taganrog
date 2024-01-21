use axum::{Extension, Router};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
pub use error::{ApiError};
use crate::config::Config;
use crate::db::DbRepo;
use crate::api::controllers::{media, ping, tags};

mod error;
mod controllers;

pub const CONTENT_TYPE_HEADER: &str = "content-type";
pub const APPLICATION_JSON: &str = "application/json";

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    pub cfg: Arc<Config>,
    pub db: DbRepo,
}

impl ApiContext {
    pub fn new(config: Config, db: DbRepo) -> Self {
        Self {
            cfg: Arc::new(config),
            db,
        }
    }
}

pub async fn serve(ctx: ApiContext) {
    let app = api_router()
        .layer(CorsLayer::new()
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_origin(Any))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(ctx))
                .layer(TraceLayer::new_for_http()),
    );

    let addr = "[::]:1698";
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind to address");
    info!("listening on {}", &addr);
    axum::serve(listener, app).await.expect("error running HTTP server");
}

fn api_router() -> Router {
    ping::router()
        .merge(media::router())
        .merge(tags::router())
}
