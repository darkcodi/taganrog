use crate::http::{ApiContext, Result};
use axum::extract::{Extension};
use axum::routing::{get};
use axum::{Json, Router};
use crate::db::surreal_http::SurrealDbResult;

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

