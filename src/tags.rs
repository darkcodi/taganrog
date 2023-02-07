use axum::extract::State;
use axum::http::StatusCode;
use sqlx::PgPool;

pub async fn test_db(
    State(pool): State<PgPool>,
) -> Result<String, (StatusCode, String)> {
    sqlx::query_scalar("select 'db ok'")
        .fetch_one(&pool)
        .await
        .map_err(internal_error)
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
    where
        E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
