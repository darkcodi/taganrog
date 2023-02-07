use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::NaiveDateTime;
use sqlx::PgPool;
use sqlx::FromRow;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Serialize, Debug, FromRow)]
pub struct Tag {
    id: i64,
    name: String,
    created_at: NaiveDateTime,
}

#[derive(Deserialize)]
pub struct CreateTagParams {
    name: String,
}

pub async fn test_db(
    State(pool): State<PgPool>,
) -> Result<String, (StatusCode, String)> {
    sqlx::query_scalar("select 'db ok'")
        .fetch_one(&pool)
        .await
        .map_err(internal_error)
}

pub async fn create_tag(
    State(pool): State<PgPool>,
    Query(params): Query<CreateTagParams>,
) -> Response {
    let name = &params.name;
    let insert_result = sqlx::query_as::<_, Tag>(r#"insert into tags (name) values ($1) returning id, name, created_at"#)
        .bind(name)
        .fetch_one(&pool)
        .await;
    match insert_result {
        Ok(tag) => Json(tag).into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

pub async fn get_tag(
    State(pool): State<PgPool>,
    Path(tag_id): Path<i64>,
) -> Response {
    let fetch_result = sqlx::query_as::<_, Tag>(r#"select id, name, created_at from tags where id = $1"#)
        .bind(tag_id)
        .fetch_one(&pool)
        .await;
    match fetch_result {
        Ok(tag) => Json(tag).into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
    where
        E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
