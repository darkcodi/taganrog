use crate::config::Config;
use anyhow::Context;
use axum::{Extension, Router};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;
pub use error::{Error, ResultExt};

mod error;
mod media;
mod tags;
mod ping;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
struct ApiContext {
    config: Arc<Config>,
    db: PgPool,
}

pub async fn serve(config: Config, db: PgPool) -> anyhow::Result<()> {
    let app = api_router().layer(
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
