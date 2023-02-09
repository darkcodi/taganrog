use std::os::linux::raw::stat;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::NaiveDateTime;
use s3::{Bucket, Region};
use s3::creds::Credentials;
use sqlx::PgPool;
use sqlx::FromRow;
use serde::{Deserialize, Serialize};
use tokio::io::{self, AsyncReadExt};
use crate::AppState;

#[derive(Serialize, Debug, FromRow, Default)]
pub struct Media {
    id: i64,
    original_filename: String,
    content_type: String,
    created_at: NaiveDateTime,
}

pub async fn create_media(
    State(state): State<AppState>,
    mut files: Multipart,
) -> Response {
    let bucket = Bucket::new(
        state.s3.bucket_name.as_str(),
        Region::R2 {
            account_id: state.s3.account_id,
        },
        Credentials::new(Some(state.s3.access_key.as_str()), Some(state.s3.secret_key.as_str()), None, None, None).unwrap(),
    ).unwrap().with_path_style();

    let mut media_vec = Vec::new();
    while let Some(file) = files.next_field().await.unwrap() {
        let maybe_filename = file.file_name();
        if maybe_filename.is_none() {
            return (StatusCode::BAD_REQUEST, "no data").into_response();
        }
        let original_filename = maybe_filename.unwrap().to_string();
        let content_type = file.content_type().unwrap_or("video/mp4").to_string();
        let mut data = file.bytes().await.unwrap().to_vec();
        let response_data = bucket.put_object_stream_with_content_type(&mut data.as_slice(), original_filename.as_str(), content_type.as_str()).await;
        if response_data.is_err() {
            return internal_error(response_data.err().unwrap()).into_response();
        }

        let insert_result = sqlx::query_as::<_, Media>(r#"insert into media (original_filename, content_type) values ($1, $2) returning id, original_filename, content_type, created_at"#)
            .bind(original_filename)
            .bind(content_type)
            .fetch_one(&state.pool)
            .await;
        if insert_result.is_err() {
            return internal_error(insert_result.err().unwrap()).into_response();
        }
        let media = insert_result.unwrap();
        media_vec.push(media);
    }

    Json(media_vec).into_response()
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
    where
        E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
