use crate::config::Config;
use anyhow::Context;
use axum::{Extension, Router};
use std::sync::Arc;
use axum::http::Method;
use sea_orm::DatabaseConnection;
use tower::ServiceBuilder;
use tower_http::cors::{any, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
pub use error::{Error};

mod error;
mod media;
mod tags;
mod ping;
mod auth;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
struct ApiContext {
    config: Arc<Config>,
    db: DatabaseConnection,
}

pub async fn serve(config: Config, db: DatabaseConnection) -> anyhow::Result<()> {
    let app = api_router()
        .layer(CorsLayer::new()
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_origin(Any))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(ApiContext {
                    config: Arc::new(config),
                    db,
                }))
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
