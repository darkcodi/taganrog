use anyhow::Context;
use axum::{Extension, Router};
use std::sync::Arc;
use tokio_rusqlite::Connection;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
pub use error::{ApiError};
use crate::config::{Config, PlainConfig};
use crate::http::controllers::{media, ping, tags};

mod error;
mod controllers;

pub const CONTENT_TYPE_HEADER: &str = "content-type";
pub const APPLICATION_JSON: &str = "application/json";

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    pub cfg: Arc<Config>,
    pub db: Connection,
}

impl ApiContext {
    pub fn new(config: Config, db_conn: Connection) -> Self {
        Self {
            cfg: Arc::new(config),
            db: db_conn,
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

    let addr = "[::]:1698".parse()?;
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
