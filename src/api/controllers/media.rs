use std::ffi::OsStr;
use std::str::FromStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use path_absolutize::Absolutize;
use relative_path::PathExt;
use crate::api::error::{ApiError};
use crate::api::{ApiContext, Result};
use crate::db::entities::media::{Media, MediaId};
use crate::db::entities::tag::Tag;
use crate::utils::hash_utils::MurMurHasher;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(add_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        .route("/api/media/:media_id/add-tag", post(add_tag_to_media))
        .route("/api/media/:media_id/remove-tag", post(delete_tag_from_media))
        .route("/api/media/search", post(search_media))
        .route("/api/media/upload", post(upload_media))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

#[derive(serde::Deserialize, Debug)]
struct AddMediaRequest {
    filename: String,
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
    filename: String,
    extension: Option<String>,
    content_type: String,
    data: Vec<u8>,
    size: usize,
    hash: String,
}

impl File {
    pub async fn try_from(mut value: Multipart) -> std::result::Result<Self, ApiError> {
        let file = value.next_field()
            .await
            .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
            .ok_or(ApiError::unprocessable_entity([("file", "missing file")]))?;

        let filename = file.file_name()
            .ok_or(ApiError::unprocessable_entity([("file", "filename is empty")]))?
            .to_string();
        let extension = File::get_extension(filename.as_str());

        let content_type = file.content_type().unwrap_or("application/octet-stream").to_string();
        let data = file.bytes()
            .await
            .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
            .to_vec();
        let size = data.len();
        let hash = MurMurHasher::hash_bytes(data.as_slice());

        Ok(Self {
            filename,
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

async fn add_media(
    ctx: Extension<ApiContext>,
    Json(req): Json<AddMediaRequest>,
) -> Result<Json<Media>> {
    let filepath = std::path::Path::new(req.filename.as_str()).absolutize_from(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 1")]))?;
    let relative_path = filepath.relative_to(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 2")]))?;

    if relative_path.starts_with("..") { return Err(ApiError::unprocessable_entity([("filename", "workdir violation")])); }
    if !filepath.exists() { return Err(ApiError::unprocessable_entity([("filename", "file does not exist")])); }
    if filepath.is_dir() { return Err(ApiError::unprocessable_entity([("filename", "file is a directory")])); }

    let media = Media::from_file(&filepath, &relative_path)?;
    let media = Media::create(&media, &ctx.db).await?.safe_unwrap();
    Ok(Json(media))
}

async fn upload_media(
    ctx: Extension<ApiContext>,
    mut files: Multipart,
) -> Result<Json<Media>> {
    let file = files.next_field().await
        .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .ok_or(ApiError::unprocessable_entity([("file", "missing file")]))?;

    let filename = file.file_name()
        .ok_or(ApiError::unprocessable_entity([("file", "filename is empty")]))?
        .to_string();
    if filename.chars().any(|x| !x.is_ascii_alphanumeric() && x != '.' && x != '-' && x != '_') {
        return Err(ApiError::unprocessable_entity([("file", "filename contains invalid characters")]));
    }

    let data = file.bytes().await
        .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .to_vec();
    let hash = MurMurHasher::hash_bytes(data.as_slice());
    let existing_media = Media::get_by_hash(&hash, &ctx.db).await?;
    if existing_media.is_some() {
        return Err(ApiError::conflict(existing_media));
    }

    let mut filename = filename.clone();
    let mut suffix = 0;
    while ctx.cfg.workdir.join(filename.clone()).exists() {
        suffix += 1;
        filename = format!("dup{}-{}", suffix, filename);
    }
    let filepath = ctx.cfg.workdir.join(filename.clone());
    let relative_path = filepath.relative_to(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 2")]))?;

    std::fs::write(&filepath, data)?;
    let mut media = Media::from_file(&filepath, &relative_path)?;
    media.was_uploaded = true;
    let media = Media::create(&media, &ctx.db).await?.safe_unwrap();
    Ok(Json(media))
}

async fn get_all_media(
    ctx: Extension<ApiContext>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Media>>> {
    let page_size = pagination.page_size.unwrap_or(10).clamp(1, 50);
    let page_index = pagination.page_index.unwrap_or(0);
    let media_vec: Vec<Media> = Media::get_all(page_size, page_index, &ctx.db).await?;

    Ok(Json(media_vec))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
) -> Result<Json<Media>> {
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
    Path(media_id): Path<String>,
) -> Result<Json<Media>> {
    let media_id = MediaId::from_str(&media_id)?;
    let maybe_media = Media::delete_by_id(&media_id, &ctx.db).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();

    Ok(Json(media))
}

async fn add_tag_to_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
    Json(req): Json<TagBody>,
) -> Result<Json<Media>> {
    let media_id = MediaId::from_str(&media_id)?;
    let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let mut media = maybe_media.unwrap();
    let contains_tag = media.contains_tag(&req.name);
    if !contains_tag {
        let tag = Tag::ensure_exists(&req.name, &ctx.db).await?.safe_unwrap();
        Media::add_tag(&media_id, &tag.id, &ctx.db).await?;
        media.add_tag_str(&tag.name);
    }
    Ok(Json(media))
}

async fn delete_tag_from_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
    Json(req): Json<TagBody>,
) -> Result<Json<Media>> {
    let media_id = MediaId::from_str(&media_id)?;
    let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let mut media = maybe_media.unwrap();
    if media.contains_tag(&req.name) {
        Media::remove_tag(&media_id, &req.name, &ctx.db).await?;
        media.remove_tag_str(&req.name);
    }
    Ok(Json(media))
}

async fn search_media(
    ctx: Extension<ApiContext>,
    Json(req): Json<SearchBody>,
) -> Result<Json<Vec<Media>>> {
    if req.tags.len() == 1 && req.tags.first().unwrap() == "null" {
        let media_vec: Vec<Media> = Media::get_untagged(&ctx.db).await?;
        Ok(Json(media_vec))
    }
    else if req.tags.len() == 1 && req.tags.first().unwrap() == "all" {
        let media_vec: Vec<Media> = Media::get_all(50, 0, &ctx.db).await?;
        Ok(Json(media_vec))
    }
    else {
        let media_vec: Vec<Media> = Media::search(&req.tags, &ctx.db).await?;
        Ok(Json(media_vec))
    }
}
