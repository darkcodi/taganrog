use std::ffi::OsStr;
use amplify_derive::Wrapper;
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
use crate::entities::*;
use crate::hash::MurMurHasher;
use crate::http::error::{Error};
use crate::http::{ApiContext, Result};

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(create_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MediaResponse {
    id: i64,
    guid: Uuid,
    original_filename: String,
    extension: Option<String>,
    new_filename: String,
    content_type: String,
    created_at: NaiveDateTime,
    hash: String,
    public_url: String,
    tags: Vec<String>,
}

#[derive(Clone, Wrapper, Default, amplify_derive::From, Debug)]
struct MediaWithTags(
    #[wrap]
    #[from]
    Vec<(Media, Option<Tag>)>
);

impl From<Media> for MediaResponse {
    fn from(value: Media) -> Self {
        Self {
            id: value.id,
            guid: value.guid,
            original_filename: value.original_filename,
            extension: value.extension,
            new_filename: value.new_filename,
            content_type: value.content_type,
            created_at: value.created_at,
            hash: value.hash,
            public_url: value.public_url,
            tags: Vec::new(),
        }
    }
}

impl From<MediaWithTags> for Option<MediaResponse> {
    fn from(value: MediaWithTags) -> Self {
        if value.is_empty() {
            return None;
        }

        let mut tags_vec = Vec::with_capacity(value.0.len());
        for kvp in &value.0 {
            let tag = &kvp.1;
            if tag.is_some() {
                tags_vec.push(tag.as_ref().unwrap().name.clone());
            }
        }

        let media = value[0].0.clone();
        let mut media: MediaResponse = media.into();
        media.tags = tags_vec;
        Some(media)
    }
}

async fn create_media(
    ctx: Extension<ApiContext>,
    mut files: Multipart,
) -> Result<Json<MediaResponse>> {
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
    let media = media.insert(&ctx.db).await?.into();

    Ok(Json(media))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<i64>,
) -> Result<Json<MediaResponse>> {
    let media: MediaWithTags = MediaEntity::find_by_id(media_id)
        .find_also_linked(media_tag::MediaToTag)
        .all(&ctx.db)
        .await?
        .into();
    let media: Option<MediaResponse> = media.into();
    let media = media.ok_or(Error::NotFound)?;

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
