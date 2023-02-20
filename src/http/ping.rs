use crate::http::{ApiContext, APPLICATION_JSON, CONTENT_TYPE_HEADER, Result};
use axum::extract::{Extension};
use axum::routing::{get};
use axum::{Router};
use axum::http::{HeaderValue, StatusCode};
use axum::response::IntoResponse;

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
) -> Result<axum::response::Response> {
    let db_response = ctx.db.exec("CREATE tag
SET
    name = 'cr',
    created_at = time::now();")
        .await?;
    let mut response = (StatusCode::OK, db_response).into_response();
    response.headers_mut().insert(CONTENT_TYPE_HEADER, HeaderValue::from_static(APPLICATION_JSON));
    Ok(response)
}

