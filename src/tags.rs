use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::NaiveDateTime;
use sqlx::FromRow;
use serde::{Deserialize, Serialize};
use crate::AppState;

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

pub async fn ping() -> String {
    "pong".to_string()
}

pub async fn ping_db(
    State(state): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    sqlx::query_scalar("select 'db ok'")
        .fetch_one(&state.pool)
        .await
        .map_err(internal_error)
}

pub async fn create_tag(
    State(state): State<AppState>,
    Query(params): Query<CreateTagParams>,
) -> Response {
    let name = &params.name;
    let insert_result = sqlx::query_as::<_, Tag>(r#"insert into tags (name) values ($1) returning id, name, created_at"#)
        .bind(name)
        .fetch_one(&state.pool)
        .await;
    match insert_result {
        Ok(tag) => Json(tag).into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(tag_id): Path<i64>,
) -> Response {
    let delete_result = sqlx::query(r#"delete from tags where id = $1"#)
        .bind(tag_id)
        .execute(&state.pool)
        .await;
    match delete_result {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

pub async fn get_all_tags(
    State(state): State<AppState>,
) -> Response {
    let fetch_result = sqlx::query_as::<_, Tag>(r#"select id, name, created_at from tags"#)
        .fetch_all(&state.pool)
        .await;
    match fetch_result {
        Ok(tags) => Json(tags).into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

pub async fn get_tag_by_id(
    State(state): State<AppState>,
    Path(tag_id): Path<i64>,
) -> Response {
    let fetch_result = sqlx::query_as::<_, Tag>(r#"select id, name, created_at from tags where id = $1"#)
        .bind(tag_id)
        .fetch_one(&state.pool)
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
