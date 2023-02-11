use crate::http::{ApiContext, Result};
use axum::extract::{Extension};
use axum::routing::{get};
use axum::{Router};

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
) -> Result<String> {
    Ok(sqlx::query_scalar("select 'db ok'")
        .fetch_one(&ctx.db)
        .await?)
}

