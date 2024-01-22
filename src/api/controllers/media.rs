use std::str::FromStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path};
use axum::routing::{get, post};
use axum::{Json, Router};
use path_absolutize::Absolutize;
use relative_path::PathExt;
use crate::db::entities::{MappedMedia, Media, MediaId, Tag, TagId};
use crate::api::error::{ApiError};
use crate::api::{ApiContext, Result};
use crate::db::hash::MurMurHasher;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(add_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        .route("/api/media/:media_id/add-tag", post(add_tag_to_media))
        .route("/api/media/:media_id/remove-tag", post(delete_tag_from_media))
        .route("/api/media/upload", post(upload_media))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

#[derive(serde::Deserialize, Debug)]
struct AddMediaRequest {
    filename: String,
}

#[derive(serde::Deserialize, Debug, Default)]
struct TagBody {
    tag_name: String,
}

#[derive(serde::Deserialize, Debug, Default)]
struct SearchBody {
    tags: Vec<String>,
}

async fn add_media(
    ctx: Extension<ApiContext>,
    Json(req): Json<AddMediaRequest>,
) -> Result<Json<MappedMedia>> {
    let filepath = std::path::Path::new(req.filename.as_str()).absolutize_from(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 1")]))?;
    let relative_path = filepath.relative_to(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 2")]))?;

    if relative_path.starts_with("..") { return Err(ApiError::unprocessable_entity([("filename", "workdir violation")])); }
    if !filepath.exists() { return Err(ApiError::unprocessable_entity([("filename", "file does not exist")])); }
    if filepath.is_dir() { return Err(ApiError::unprocessable_entity([("filename", "file is a directory")])); }

    let media = Media::from_file(&filepath, &relative_path)?;
    let existing_media = ctx.db.get_media_by_hash(media.hash.clone())?;
    if existing_media.is_some() {
        let mapped_media = ctx.db.map_media(existing_media.unwrap())?;
        return Err(ApiError::conflict(mapped_media));
    }

    ctx.db.insert_media(&media)?;
    let tag = ctx.db.get_untagged_tag()?;
    let media = ctx.db.add_tag_to_media(media, tag)?;
    let mapped_media = ctx.db.map_media(media)?;
    Ok(Json(mapped_media))
}

async fn upload_media(
    ctx: Extension<ApiContext>,
    mut files: Multipart,
) -> Result<Json<MappedMedia>> {
    let file = files.next_field().await
        .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .ok_or(ApiError::unprocessable_entity([("file", "missing file")]))?;

    let original_filename = file.file_name()
        .ok_or(ApiError::unprocessable_entity([("file", "filename is empty")]))?
        .to_string();
    if original_filename.chars().any(|x| !x.is_ascii_alphanumeric() && x != '.' && x != '-' && x != '_') {
        return Err(ApiError::unprocessable_entity([("file", "filename contains invalid characters")]));
    }

    let data = file.bytes().await
        .map_err(|x| ApiError::unprocessable_entity([("file", format!("multipart error: {}", x.to_string()))]))?
        .to_vec();
    let hash = MurMurHasher::hash_bytes(data.as_slice());
    let existing_media = ctx.db.get_media_by_hash(hash)?;
    if existing_media.is_some() {
        let mapped_media = ctx.db.map_media(existing_media.unwrap())?;
        return Err(ApiError::conflict(mapped_media));
    }

    let mut filename = original_filename.clone();
    let mut suffix = 0;
    while ctx.cfg.workdir.join(filename.clone()).exists() {
        suffix += 1;
        filename = format!("dup{}-{}", suffix, original_filename);
    }
    let filepath = ctx.cfg.workdir.join(filename.clone());
    let relative_path = filepath.relative_to(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 2")]))?;

    std::fs::write(&filepath, data)?;
    let mut media = Media::from_file(&filepath, &relative_path)?;
    media.was_uploaded = true;
    ctx.db.insert_media(&media)?;
    let tag = ctx.db.get_untagged_tag()?;
    let media = ctx.db.add_tag_to_media(media, tag)?;
    let mapped_media = ctx.db.map_media(media)?;
    Ok(Json(mapped_media))
}

async fn get_all_media(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<MappedMedia>>> {
    let media_vec: Vec<Media> = ctx.db.get_all_media()?;
    let mapped_media = ctx.db.map_many_media(media_vec)?;
    Ok(Json(mapped_media))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
) -> Result<Json<MappedMedia>> {
    let media_id = MediaId::from_str(&media_id)
        .map_err(|_| ApiError::unprocessable_entity([("media_id", "invalid id")]))?;
    let maybe_media = ctx.db.get_media_by_id(media_id)?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    let mapped_media = ctx.db.map_media(media)?;
    Ok(Json(mapped_media))
}

async fn delete_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
) -> Result<Json<MappedMedia>> {
    let media_id = MediaId::from_str(&media_id)
        .map_err(|_| ApiError::unprocessable_entity([("media_id", "invalid id")]))?;
    let maybe_media = ctx.db.get_media_by_id(media_id.clone())?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    ctx.db.delete_media(&media)?;
    let mapped_media = ctx.db.map_media(media)?;
    Ok(Json(mapped_media))
}

async fn add_tag_to_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
    Json(req): Json<TagBody>,
) -> Result<Json<MappedMedia>> {
    let media_id = MediaId::from_str(&media_id)
        .map_err(|_| ApiError::unprocessable_entity([("media_id", "invalid id")]))?;
    let maybe_media = ctx.db.get_media_by_id(media_id.clone())?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    if req.tag_name == Tag::untagged().name {
        return Err(ApiError::unprocessable_entity([("tag", "cannot add untagged tag")]));
    }

    let media = maybe_media.unwrap();
    let tag = ctx.db.get_or_insert_tag_by_name(req.tag_name.clone())?.unwrap();
    let mut media = ctx.db.add_tag_to_media(media, tag)?;
    if media.tags.iter().any(|x| x == &TagId::untagged()) {
        let tag = ctx.db.get_untagged_tag()?;
        media = ctx.db.delete_tag_from_media(media, tag)?;
    }
    let mapped_media = ctx.db.map_media(media)?;
    Ok(Json(mapped_media))
}

async fn delete_tag_from_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
    Json(req): Json<TagBody>,
) -> Result<Json<MappedMedia>> {
    let media_id = MediaId::from_str(&media_id)
        .map_err(|_| ApiError::unprocessable_entity([("media_id", "invalid id")]))?;
    let maybe_media = ctx.db.get_media_by_id(media_id.clone())?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    if req.tag_name == Tag::untagged().name {
        return Err(ApiError::unprocessable_entity([("tag", "cannot remove untagged tag")]));
    }

    let maybe_tag = ctx.db.get_tag_by_name(req.tag_name.clone())?;
    if maybe_tag.is_none() {
        return Err(ApiError::NotFound);
    }

    let tag = maybe_tag.unwrap();
    let media = maybe_media.unwrap();
    let mut media = ctx.db.delete_tag_from_media(media, tag)?;
    if media.tags.len() == 0 {
        let tag = ctx.db.get_untagged_tag()?;
        media = ctx.db.add_tag_to_media(media, tag)?;
    }
    let mapped_media = ctx.db.map_media(media)?;
    Ok(Json(mapped_media))
}
