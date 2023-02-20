use crate::http::{ApiContext, APPLICATION_JSON, CONTENT_TYPE_HEADER, Result};
use axum::extract::{Extension};
use axum::routing::{get};
use axum::{Json, Router};
use axum::http::{HeaderValue, StatusCode};
use axum::response::IntoResponse;
use crate::db::{RemoveFirst, SurrealDbResult, SurrealDeserializable};
use crate::entities::tag::Tag;

pub fn router() -> Router {
    Router::new()
        .route("/api/ping", get(ping))
        .route("/api/ping_db", get(ping_db))
}

async fn ping() -> String {
    "pong".to_string()
}

async fn ping_db(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<SurrealDbResult>>> {
    let res = ctx.db.exec("INFO FOR DB;").await?;
    Ok(Json(res))
}

