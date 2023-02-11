use crate::http::{ApiContext, Result};
use axum::extract::{Extension};
use axum::routing::{get};
use axum::{Router};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

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
    let db_response: String = ctx.db
        .query_one(Statement::from_string(DatabaseBackend::Postgres, "SELECT 'db ok' AS DbResponse".to_string()))
        .await?
        .unwrap()
        .try_get("", "DbResponse")?;
    Ok(db_response)
}

