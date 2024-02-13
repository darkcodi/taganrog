use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum::body::Body;
use axum::response::Response;
use path_absolutize::Absolutize;
use relative_path::PathExt;
use crate::api::error::{ApiError};
use crate::api::{ApiContext, Result};
use crate::db;
use crate::db::{Media, TagsAutocomplete};
use crate::utils::hash_utils::MurMurHasher;
use crate::utils::normalize_query;
use crate::utils::str_utils::StringExtensions;

const PLACEHOLDER: &[u8] = include_bytes!("assets/placeholder.svg");
const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/ping", get(ping))
        .route("/api/media", get(get_all_media).post(add_media))
        .route("/api/media/random", get(get_random_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        .route("/api/media/:media_id/thumbnail", get(get_media_thumbnail))
        .route("/api/media/:media_id/stream", get(stream_media))
        .route("/api/media/:media_id/add-tag", post(add_tag_to_media))
        .route("/api/media/:media_id/remove-tag", post(delete_tag_from_media))
        .route("/api/media/search", post(search_media))
        .route("/api/media/upload", post(upload_media))
        .route("/api/tags/autocomplete", post(autocomplete_tags))
        .route("/api/export", get(export_db))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

async fn ping() -> String {
    "pong".to_string()
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
    let thumbnail_filename = format!("{}.png", &hash);
    let thumbnail_filepath = ctx.cfg.thumbnails_dir.join(&thumbnail_filename);
    let thumbnail_exists = thumbnail_filepath.exists();
    if existing_media.is_some() && thumbnail_exists {
        return Ok(Json(existing_media.unwrap()));
    }

    let thumbnail = files.next_field().await
        .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .ok_or(ApiError::unprocessable_entity([("file", "missing thumbnail")]))?;
    let thumbnail_data = thumbnail.bytes().await
        .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .to_vec();
    if existing_media.is_some() {
        std::fs::write(&thumbnail_filepath, thumbnail_data)?;
        return Ok(Json(existing_media.unwrap()));
    }
    let mut filename = filename.clone();
    let mut suffix = 0;
    while ctx.cfg.workdir.join(filename.clone()).exists() {
        suffix += 1;
        filename = format!("dup{}-{}", suffix, filename);
    }
    let filepath = ctx.cfg.upload_dir.join(filename.clone());
    let relative_path = filepath.relative_to(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 2")]))?;

    std::fs::write(&thumbnail_filepath, thumbnail_data)?;
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

async fn get_random_media(
    ctx: Extension<ApiContext>,
) -> Result<Json<Media>> {
    let maybe_media = db::get_random_media(&ctx).await?;
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

    let thumbnail_path = ctx.cfg.thumbnails_dir.join(format!("{}.png", &media_id));
    if thumbnail_path.exists() {
        std::fs::remove_file(&thumbnail_path)?;
    }

    let media = maybe_media.unwrap();
    Ok(Json(media))
}

async fn get_media_thumbnail(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
) -> Result<axum::http::Response<Body>> {
    let maybe_media = db::get_media_by_id(&ctx, &media_id).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    let full_path = ctx.cfg.thumbnails_dir.join(format!("{}.png", &media.id));
    if !full_path.exists() {
        let mut response = Response::<Body>::new(PLACEHOLDER.into());
        response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
        response.headers_mut().insert("Content-Type", "image/svg+xml".parse().unwrap());
        return Ok(response);
    }

    let file = tokio::fs::File::open(&full_path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let response = Response::new(Body::from_stream(stream));
    Ok(response)
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
    let page_size = req.s.unwrap_or(10).clamp(1, 50);
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
    if query == "no-thumbnail" {
        let media_vec = db::get_media_without_thumbnail(&ctx).await?;
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
) -> Result<axum::http::Response<Body>> {
    let maybe_media = db::get_media_by_id(&ctx, &media_id).await?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    let full_path = ctx.cfg.workdir.join(&media.location);
    let file = tokio::fs::File::open(&full_path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let response = Response::new(Body::from_stream(stream));
    Ok(response)
}

async fn export_db(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Media>>> {
    let media_vec = db::get_all_media(&ctx, u64::MAX, 0).await?;
    Ok(Json(media_vec))
}
