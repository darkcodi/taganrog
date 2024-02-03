use std::ffi::OsStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use path_absolutize::Absolutize;
use relative_path::PathExt;
use crate::api::error::{ApiError};
use crate::api::{ApiContext, Result};
use crate::db;
use crate::db::{Media, TagsAutocomplete};
use crate::utils::hash_utils::MurMurHasher;
use crate::utils::str_utils::StringExtensions;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(add_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        .route("/api/media/:media_id/stream", get(stream_media))
        .route("/api/media/:media_id/add-tag", post(add_tag_to_media))
        .route("/api/media/:media_id/remove-tag", post(delete_tag_from_media))
        .route("/api/media/search", post(search_media))
        .route("/api/media/upload", post(upload_media))
        .route("/api/tags/autocomplete", post(autocomplete_tags))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

#[derive(serde::Deserialize, Debug)]
struct AddMediaRequest {
    filename: String,
    created_at: Option<u64>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct TagBody {
    name: String,
}

#[derive(serde::Deserialize, Debug, Default)]
struct SearchBody {
    q: String,
    p: Option<u64>,
    s: Option<u64>,
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

    let media = Media::from_file(&filepath, &relative_path, req.created_at)?;
    let media = db::create_media(&ctx, media).await?;
    Ok(Json(media.safe_unwrap()))
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
    let existing_media = db::get_media_by_id(&ctx, &hash).await?;
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
    let mut media = Media::from_file(&filepath, &relative_path, None)?;
    media.was_uploaded = true;
    media = db::create_media(&ctx, media).await?.safe_unwrap();
    Ok(Json(media))
}

async fn get_all_media(
    ctx: Extension<ApiContext>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Media>>> {
    let page_size = pagination.page_size.unwrap_or(10).clamp(1, 50);
    let page_index = pagination.page_index.unwrap_or(0);
    let media_vec = db::get_all_media(&ctx, page_size, page_index).await?;
    Ok(Json(media_vec))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
) -> Result<Json<Media>> {
    let maybe_media = db::get_media_by_id(&ctx, &media_id).await?;
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
    let maybe_media = db::delete_media_by_id(&ctx, &media_id).await?;
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
    let tag = req.name.slugify();
    let maybe_media = db::get_media_by_id(&ctx, &media_id).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let mut media = maybe_media.unwrap();
    let contains_tag = media.tags.contains(&tag);
    if !contains_tag {
        db::add_tag_to_media(&ctx, &media_id, &tag).await?;
        media.tags.push(tag);
    }
    Ok(Json(media))
}

async fn delete_tag_from_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
    Json(req): Json<TagBody>,
) -> Result<Json<Media>> {
    let tag = req.name.slugify();
    let maybe_media = db::get_media_by_id(&ctx, &media_id).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let mut media = maybe_media.unwrap();
    let contains_tag = media.tags.contains(&tag);
    if contains_tag {
        db::remove_tag_from_media(&ctx, &media_id, &tag).await?;
        media.tags.retain(|x| x != &tag);
    }
    Ok(Json(media))
}

async fn search_media(
    ctx: Extension<ApiContext>,
    Json(req): Json<SearchBody>,
) -> Result<Json<Vec<Media>>> {
    let page_size = req.s.unwrap_or(5).clamp(1, 50);
    let page_index = req.p.unwrap_or(0);
    let query = normalize_query(&req.q);
    if query.is_empty() {
        return Ok(Json(vec![]));
    }
    if query == "null" {
        let media_vec = db::get_untagged_media(&ctx, page_size, page_index).await?;
        return Ok(Json(media_vec))
    }
    if query == "all" {
        let media_vec = db::get_all_media(&ctx, page_size, page_index).await?;
        return Ok(Json(media_vec))
    }
    let media_vec = db::search_media(&ctx, &query, page_size, page_index).await?;
    Ok(Json(media_vec))
}

async fn autocomplete_tags(
    ctx: Extension<ApiContext>,
    Json(req): Json<SearchBody>,
) -> Result<Json<Vec<TagsAutocomplete>>> {
    let page_size = req.s.unwrap_or(10).clamp(1, 50);
    let query = normalize_query(&req.q);
    if query.is_empty() {
        return Ok(Json(vec![]));
    }
    let autocomplete = db::autocomplete_tags(&ctx, &query, page_size as usize).await?;
    Ok(Json(autocomplete))
}

async fn stream_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
) -> Result<axum::http::Response<axum::body::Body>> {
    let maybe_media = db::get_media_by_id(&ctx, &media_id).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    let full_path = ctx.cfg.workdir.join(&media.location);
    let file = tokio::fs::File::open(&full_path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let response = axum::http::Response::new(axum::body::Body::from_stream(stream));
    Ok(response)
}

fn normalize_query(query: &str) -> String {
    let mut tags = query.split(" ").map(|x| x.trim()).filter(|x| !x.is_empty()).map(|x| x.slugify().to_string()).collect::<Vec<String>>();
    if tags.len() > 0 && query.ends_with(" ") {
        tags.push("".to_string());
    }
    tags.join(" ")
}
