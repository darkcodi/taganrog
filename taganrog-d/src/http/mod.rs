use crate::config::Config;
use anyhow::Context;
use axum::{Extension, Router};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
pub use error::{ApiError};
use crate::db::surreal_http::SurrealHttpClient;
use crate::http::controllers::{media, ping, tags};

mod error;
mod controllers;

pub const CONTENT_TYPE_HEADER: &str = "content-type";
pub const APPLICATION_JSON: &str = "application/json";

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    pub cfg: Arc<Config>,
    pub db: SurrealHttpClient,
}

impl ApiContext {
    pub fn new(config: Config) -> Self {
        let cfg = Arc::new(config);
        let db = SurrealHttpClient::new(cfg.db.database_url.as_str(), "root", "root", "tg1", "tg1");
        Self {
            cfg,
            db,
        }
    }
}

pub async fn serve(ctx: ApiContext) -> anyhow::Result<()> {
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

    let addr = "[::]:3000".parse()?;
    info!("listening on {}", &addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("error running HTTP server")
}

fn api_router() -> Router {
    ping::router()
        .merge(media::router())
        .merge(tags::router())
}
