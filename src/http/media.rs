use std::ffi::OsStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path};
use axum::routing::{get};
use axum::{Json, Router};
use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use s3::{Bucket, Region};
use s3::creds::Credentials;
use sea_orm::{Set};
use uuid::Uuid;
use crate::config::S3Configuration;
use crate::hash::MurMurHasher;
use crate::http::error::{Error};
use crate::http::{ApiContext, Result};
use crate::http::media::Entity as MediaEntity;
use crate::http::media::Column as MediaColumn;
use crate::http::media::Model as Media;
use crate::http::media::ActiveModel as ActiveMedia;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(create_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub guid: Uuid,
    pub original_filename: String,
    pub extension: Option<String>,
    pub new_filename: String,
    pub content_type: String,
    pub created_at: NaiveDateTime,
    pub hash: String,
    pub public_url: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
}

impl ActiveModelBehavior for ActiveModel {}

async fn create_media(
    ctx: Extension<ApiContext>,
    mut files: Multipart,
) -> Result<Json<Media>> {
    let file = files.next_field()
        .await
        .map_err(|x| Error::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .ok_or(Error::unprocessable_entity([("file", "missing file")]))?;

    let original_filename = file.file_name()
        .ok_or(Error::unprocessable_entity([("file", "filename is empty")]))?
        .to_string();

    let content_type = file.content_type().unwrap_or("application/octet-stream").to_string();
    let data = file.bytes()
        .await
        .map_err(|x| Error::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .to_vec();
    let hash = MurMurHasher::hash_bytes(data.as_slice());

    let existing_media = MediaEntity::find()
        .filter(MediaColumn::Hash.eq(&hash))
        .one(&ctx.db)
        .await?;
    if existing_media.is_some() {
        return Err(Error::unprocessable_entity([("hash", "media with such hash exists")]));
    }

    let guid = Uuid::new_v4();
    let extension = get_extension_from_filename(original_filename.as_str());
    let new_filename = format!("{}{}", guid, extension.clone().unwrap_or("".to_string()));
    let bucket = get_bucket(&ctx.config.s3);
    bucket.put_object_stream_with_content_type(&mut data.as_slice(), new_filename.as_str(), content_type.as_str())
        .await
        .map_err(Error::S3Error)?;

    let created_at = chrono::Utc::now().naive_utc();
    let public_url = format!("{}{}", &ctx.config.s3.public_url_prefix, new_filename);

    let media = ActiveMedia {
        guid: Set(guid),
        original_filename: Set(original_filename),
        extension: Set(extension),
        new_filename: Set(new_filename),
        content_type: Set(content_type),
        created_at: Set(created_at),
        hash: Set(hash),
        public_url: Set(public_url),
        ..Default::default()
    };
    let media = media.insert(&ctx.db).await?;

    Ok(Json(media))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<i64>,
) -> Result<Json<Media>> {
    let media = MediaEntity::find_by_id(media_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    Ok(Json(media))
}

async fn get_all_media(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Media>>> {
    let media_vec: Vec<Media> = MediaEntity::find()
        .all(&ctx.db)
        .await?;

    Ok(Json(media_vec))
}

async fn delete_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<i64>,
) -> Result<()> {
    let media = MediaEntity::find_by_id(media_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    let bucket = get_bucket(&ctx.config.s3);
    bucket.delete_object(&media.new_filename).await?;

    media.delete(&ctx.db).await?;

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

fn get_extension_from_filename(filename: &str) -> Option<String> {
    std::path::Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
        .map(|x| format!(".{x}"))
}
