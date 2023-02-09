use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::NaiveDateTime;
use s3::{Bucket, Region};
use s3::creds::Credentials;
use sqlx::FromRow;
use serde::{Serialize};
use crate::{AppState, S3Configuration};
use crate::hash::MurMurHasher;

#[derive(Serialize, Debug, FromRow, Default)]
pub struct Media {
    id: i64,
    original_filename: String,
    content_type: String,
    created_at: NaiveDateTime,
    hash: String,
    public_url: String,
}

pub async fn create_media(
    State(state): State<AppState>,
    mut files: Multipart,
) -> Response {
    let bucket = get_bucket(&state.s3);

    let mut media_vec = Vec::new();
    while let Some(file) = files.next_field().await.unwrap() {
        let maybe_filename = file.file_name();
        if maybe_filename.is_none() {
            return (StatusCode::BAD_REQUEST, "no data").into_response();
        }
        let original_filename = maybe_filename.unwrap().to_string();
        let content_type = file.content_type().unwrap_or("video/mp4").to_string();
        let data = file.bytes().await.unwrap().to_vec();
        let hash = MurMurHasher::hash_bytes(data.as_slice());
        let public_url = format!("{}{}", &state.s3.public_url_prefix, original_filename);
        let response_data = bucket.put_object_stream_with_content_type(&mut data.as_slice(), original_filename.as_str(), content_type.as_str()).await;
        if response_data.is_err() {
            return internal_error(response_data.err().unwrap()).into_response();
        }

        let insert_result = sqlx::query_as::<_, Media>(r#"insert into media (original_filename, content_type, hash, public_url) values ($1, $2, $3, $4) returning id, original_filename, content_type, created_at, hash, public_url"#)
            .bind(original_filename)
            .bind(content_type)
            .bind(hash)
            .bind(public_url)
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

pub async fn read_media(
    State(state): State<AppState>,
    Path(media_id): Path<i64>,
) -> Response {
    let bucket = get_bucket(&state.s3);
    todo!()
}

fn get_bucket(conf: &S3Configuration) -> Bucket {
    let bucket = Bucket::new(
        conf.bucket_name.as_str(),
        Region::R2 {
            account_id: conf.account_id.clone(),
        },
        Credentials::new(Some(conf.access_key.as_str()), Some(conf.secret_key.as_str()), None, None, None).unwrap(),
    ).unwrap().with_path_style();
    bucket
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
    where
        E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
