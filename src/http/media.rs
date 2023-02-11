use crate::http::{ApiContext, Result};
use axum::extract::{Extension, Multipart, Path};
use axum::routing::{get};
use axum::{Json, Router};
use chrono::NaiveDateTime;
use s3::{Bucket, Region};
use s3::creds::Credentials;
use sqlx::FromRow;
use crate::config::S3Configuration;
use crate::hash::MurMurHasher;

use crate::http::error::{Error, ResultExt};

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(create_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, FromRow)]
pub struct Media {
    id: i64,
    original_filename: String,
    content_type: String,
    created_at: NaiveDateTime,
    hash: String,
    public_url: String,
}

async fn create_media(
    ctx: Extension<ApiContext>,
    mut files: Multipart,
) -> Result<Json<Media>> {
    let bucket = get_bucket(&ctx.config.s3);
    let maybe_file = files.next_field().await;
    if maybe_file.is_err() {
        return Err(Error::unprocessable_entity([("file", "missing file")]));
    }

    let file = maybe_file.unwrap().unwrap();
    let maybe_filename = file.file_name();
    if maybe_filename.is_none() {
        return Err(Error::unprocessable_entity([("file", "filename is empty")]));
    }

    // TODO: check hash before uploading

    let original_filename = maybe_filename.unwrap().to_string();
    let content_type = file.content_type().unwrap_or("video/mp4").to_string();
    let data = file.bytes().await.unwrap().to_vec();
    let response_data = bucket.put_object_stream_with_content_type(&mut data.as_slice(), original_filename.as_str(), content_type.as_str()).await;
    if response_data.is_err() {
        return Err(Error::Anyhow(response_data.err().unwrap().into()));
    }

    let created_at = chrono::Utc::now().naive_utc();
    let hash = MurMurHasher::hash_bytes(data.as_slice());
    let public_url = format!("{}{}", &ctx.config.s3.public_url_prefix, original_filename);
    let id = sqlx::query_scalar(r#"insert into media (original_filename, content_type, hash, public_url, created_at) values ($1, $2, $3, $4, $5) returning id"#)
        .bind(&original_filename)
        .bind(&content_type)
        .bind(&hash)
        .bind(&created_at)
        .bind(&public_url)
        .fetch_one(&ctx.db)
        .await
        .on_constraint("media_hash", |_| {
            Error::unprocessable_entity([("hash", "media with such hash exists")])
        })?;

    Ok(Json(Media {
        id,
        original_filename,
        content_type,
        created_at,
        hash,
        public_url,
    }))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<i64>,
) -> Result<Json<Media>> {
    let media = sqlx::query_as::<_, Media>(r#"select id, original_filename, content_type, created_at, hash, public_url from media where id = $1"#)
        .bind(media_id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    Ok(Json(media))
}

async fn get_all_media(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Media>>> {
    let media_vec = sqlx::query_as::<_, Media>(r#"select id, original_filename, content_type, created_at, hash, public_url from media"#)
        .fetch_all(&ctx.db)
        .await?;

    Ok(Json(media_vec))
}

async fn delete_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<i64>,
) -> Result<()> {
    let media = sqlx::query_as::<_, Media>(r#"select id, original_filename, content_type, created_at, hash, public_url from media where id = $1"#)
        .bind(media_id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    let bucket = get_bucket(&ctx.config.s3);
    bucket.delete_object(&media.original_filename).await?;

    sqlx::query(r#"delete from media where id = $1"#)
        .bind(media_id)
        .execute(&ctx.db)
        .await?;

    Ok(())
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
