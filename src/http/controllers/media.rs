use std::ffi::OsStr;
use std::str::FromStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_auth::AuthBearer;
use chrono::{DateTime, Utc};
use s3::{Bucket, Region};
use s3::creds::Credentials;
use uuid::Uuid;
use crate::config::S3Configuration;
use crate::db::DbResult;
use crate::db::entities::media::{Media, MediaId, MediaWithTags};
use crate::db::entities::tag::Tag;
use crate::http::error::{ApiError};
use crate::http::{ApiContext, auth, Result};
use crate::http::auth::MyCustomBearerAuth;
use crate::utils::hash_utils::MurMurHasher;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(create_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        .route("/api/media/:media_id/add-tag", post(add_tag_to_media))
        .route("/api/media/:media_id/remove-tag", post(delete_tag_from_media))
        .route("/api/media/search", post(search_media))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

#[derive(serde::Deserialize, Debug, Default)]
struct TagBody {
    name: String,
}

#[derive(serde::Deserialize, Debug, Default)]
struct SearchBody {
    tags: Vec<String>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct Pagination {
    page_size: Option<u64>,
    page_index: Option<u64>,
}

struct File {
    original_filename: String,
    extension: Option<String>,
    content_type: String,
    data: Vec<u8>,
    size: usize,
    hash: String,
}

impl File {
    pub async fn try_from(mut value: Multipart) -> Result<Self, ApiError> {
        let file = value.next_field()
            .await
            .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
            .ok_or(ApiError::unprocessable_entity([("file", "missing file")]))?;

        let original_filename = file.file_name()
            .ok_or(ApiError::unprocessable_entity([("file", "filename is empty")]))?
            .to_string();
        let extension = File::get_extension(original_filename.as_str());

        let content_type = file.content_type().unwrap_or("application/octet-stream").to_string();
        let data = file.bytes()
            .await
            .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
            .to_vec();
        let size = data.len();
        let hash = MurMurHasher::hash_bytes(data.as_slice());

        Ok(Self {
            original_filename,
            extension,
            content_type,
            data,
            size,
            hash,
        })
    }

    fn get_extension(filename: &str) -> Option<String> {
        std::path::Path::new(filename)
            .extension()
            .and_then(OsStr::to_str)
            .map(|x| format!(".{x}"))
    }
}

async fn create_media(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    files: Multipart,
) -> Result<Json<MediaWithTags>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    let file = File::try_from(files).await?;
    let existing_media = Media::get_by_hash(&file.hash, &ctx.db).await?;
    if existing_media.is_some() {
        return Err(ApiError::conflict(existing_media.unwrap()));
    }

    let id = MediaId::new();
    let new_filename = format!("{}{}", &file.hash, &file.extension.clone().unwrap_or("".to_string()));
    let bucket = get_bucket(&ctx.cfg.s3);
    bucket.put_object_stream_with_content_type(&mut file.data.as_slice(), new_filename.as_str(), &file.content_type.as_str())
        .await
        .map_err(ApiError::S3Error)?;

    let created_at = DateTime::<Utc>::from_utc(Utc::now().naive_utc(), Utc);
    let public_url = format!("{}{}", &ctx.cfg.s3.public_url_prefix, new_filename);

    let media = Media {
        id: id,
        original_filename: file.original_filename,
        extension: file.extension,
        new_filename,
        content_type: file.content_type,
        created_at,
        hash: file.hash,
        size: file.size as i64,
        public_url,
    };
    let media = Media::create(&media, &ctx.db).await?;
    Ok(Json(media))
}


async fn get_all_media(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<MediaWithTags>>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;
    let page_size = pagination.page_size.unwrap_or(10).clamp(1, 50);
    let page_index = pagination.page_index.unwrap_or(0);
    let media_vec: Vec<MediaWithTags> = Media::get_all(page_size, page_index, &ctx.db).await?;

    Ok(Json(media_vec))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(media_id): Path<String>,
) -> Result<Json<MediaWithTags>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    let media_id = MediaId::from_str(&media_id)?;
    let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    Ok(Json(media))
}

async fn delete_media(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(media_id): Path<String>,
) -> Result<Json<Media>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    let media_id = MediaId::from_str(&media_id)?;
    let maybe_media = Media::delete_by_id(&media_id, &ctx.db).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    let bucket = get_bucket(&ctx.cfg.s3);
    bucket.delete_object(&media.new_filename).await?;

    Ok(Json(media))
}

async fn add_tag_to_media(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(media_id): Path<String>,
    Json(req): Json<TagBody>,
) -> Result<Json<MediaWithTags>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    let media_id = MediaId::from_str(&media_id)?;
    let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let mut media = maybe_media.unwrap();
    if !media.tags.contains(&req.name) {
        let tag = Tag::ensure_exists(&req.name, &ctx.db).await?.safe_unwrap();
        Media::add_tag(&media_id, &tag.id, &ctx.db).await?;
        media.tags.push(tag.name);
    }
    Ok(Json(media))
}

async fn delete_tag_from_media(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(media_id): Path<String>,
    Json(req): Json<TagBody>,
) -> Result<Json<MediaWithTags>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    let media_id = MediaId::from_str(&media_id)?;
    let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let mut media = maybe_media.unwrap();
    if media.tags.contains(&req.name) {
        Media::remove_tag(&media_id, &req.name, &ctx.db).await?;
        let pos = media.tags.iter().position(|x| x == &req.name).unwrap();
        media.tags.remove(pos);
    }
    Ok(Json(media))
}

async fn search_media(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Json(req): Json<SearchBody>,
) -> Result<Json<Vec<MediaWithTags>>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;
    if req.tags.len() == 1 && req.tags.first().unwrap() == "null" {
        let media_vec: Vec<MediaWithTags> = Media::get_untagged(&ctx.db).await?;
        Ok(Json(media_vec))
    }
    else if req.tags.len() == 1 && req.tags.first().unwrap() == "all" {
        let media_vec: Vec<MediaWithTags> = Media::get_all(50, 0, &ctx.db).await?;
        Ok(Json(media_vec))
    }
    else {
        let media_vec: Vec<MediaWithTags> = Media::search(&req.tags, &ctx.db).await?;
        Ok(Json(media_vec))
    }
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
