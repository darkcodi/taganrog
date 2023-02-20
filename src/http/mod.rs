use crate::config::Config;
use anyhow::Context;
use axum::{Extension, Router};
use std::sync::Arc;
use surrealdb::Datastore;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
pub use error::{Error};
use crate::db::DbContext;

mod error;
mod media;
mod tags;
mod ping;
mod auth;

pub const CONTENT_TYPE_HEADER: &str = "content-type";
pub const APPLICATION_JSON: &str = "application/json";

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    cfg: Arc<Config>,
    db: DbContext,
}

impl ApiContext {
    fn new(config: Config, db: Datastore) -> Self {
        Self {
            cfg: Arc::new(config),
            db: DbContext::new(db),
        }
    }
}

pub async fn serve(config: Config, db: Datastore) -> anyhow::Result<()> {
    let app = api_router()
        .layer(CorsLayer::new()
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_origin(Any))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(ApiContext::new(config, db)))
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
