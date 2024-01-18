use std::str::FromStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path};
use axum::routing::{get, post};
use axum::{Json, Router};
use path_absolutize::Absolutize;
use relative_path::PathExt;
use crate::db::entities::{Media, MediaId};
use crate::http::error::{ApiError};
use crate::http::{ApiContext, Result};
use crate::utils::hash_utils::MurMurHasher;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(create_media))
        .route("/api/media/:media_id", get(get_media).delete(delete_media))
        // .route("/api/media/:media_id/add-tag", post(add_tag_to_media))
        // .route("/api/media/:media_id/remove-tag", post(delete_tag_from_media))
        // .route("/api/media/search", post(search_media))
        .route("/api/media/upload", post(upload_media))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
}

#[derive(serde::Deserialize, Debug)]
struct ImportMediaRequest {
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

async fn create_media(
    ctx: Extension<ApiContext>,
    Json(req): Json<ImportMediaRequest>,
) -> Result<Json<Media>> {
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
        return Err(ApiError::conflict(existing_media.unwrap()));
    }

    ctx.db.insert_media(&media)?;
    Ok(Json(media))
}

async fn upload_media(
    ctx: Extension<ApiContext>,
    mut files: Multipart,
) -> Result<Json<Media>> {
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
        return Err(ApiError::conflict(existing_media.unwrap()));
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
    Ok(Json(media))
}

async fn get_all_media(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Media>>> {
    let media_vec: Vec<Media> = ctx.db.get_all_media()?;
    Ok(Json(media_vec))
}

async fn get_media(
    ctx: Extension<ApiContext>,
    Path(media_id): Path<String>,
) -> Result<Json<Media>> {
    let media_id = MediaId::from_str(&media_id)
        .map_err(|_| ApiError::unprocessable_entity([("media_id", "invalid id")]))?;
    let maybe_media = ctx.db.get_media_by_id(media_id)?;
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
    let media_id = MediaId::from_str(&media_id)
        .map_err(|_| ApiError::unprocessable_entity([("media_id", "invalid id")]))?;
    let maybe_media = ctx.db.get_media_by_id(media_id.clone())?;
    if maybe_media.is_none() {
        return Err(ApiError::NotFound);
    }

    let media = maybe_media.unwrap();
    ctx.db.delete_media(&media)?;
    Ok(Json(media))
}

// async fn add_tag_to_media(
//     ctx: Extension<ApiContext>,
//     Path(media_id): Path<String>,
//     Json(req): Json<TagBody>,
// ) -> Result<Json<MediaWithTags>> {
//     let media_id = MediaId::from_str(&media_id)?;
//     let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
//     if maybe_media.is_none() {
//         return Err(ApiError::NotFound);
//     }
//
//     let mut media = maybe_media.unwrap();
//     if !media.tags.contains(&req.name) {
//         let tag = Tag::ensure_exists(&req.name, &ctx.db).await?.safe_unwrap();
//         Media::add_tag(&media_id, &tag.id, &ctx.db).await?;
//         media.tags.push(tag.name);
//     }
//     Ok(Json(media))
// }
//
// async fn delete_tag_from_media(
//     ctx: Extension<ApiContext>,
//     Path(media_id): Path<String>,
//     Json(req): Json<TagBody>,
// ) -> Result<Json<MediaWithTags>> {
//     let media_id = MediaId::from_str(&media_id)?;
//     let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
//     if maybe_media.is_none() {
//         return Err(ApiError::NotFound);
//     }
//
//     let mut media = maybe_media.unwrap();
//     if media.tags.contains(&req.name) {
//         Media::remove_tag(&media_id, &req.name, &ctx.db).await?;
//         let pos = media.tags.iter().position(|x| x == &req.name).unwrap();
//         media.tags.remove(pos);
//     }
//     Ok(Json(media))
// }
//
// async fn search_media(
//     ctx: Extension<ApiContext>,
//     Json(req): Json<SearchBody>,
// ) -> Result<Json<Vec<MediaWithTags>>> {
//     if req.tags.len() == 1 && req.tags.first().unwrap() == "null" {
//         let media_vec: Vec<MediaWithTags> = Media::get_untagged(&ctx.db).await?;
//         Ok(Json(media_vec))
//     }
//     else if req.tags.len() == 1 && req.tags.first().unwrap() == "all" {
//         let media_vec: Vec<MediaWithTags> = Media::get_all(50, 0, &ctx.db).await?;
//         Ok(Json(media_vec))
//     }
//     else {
//         let media_vec: Vec<MediaWithTags> = Media::search(&req.tags, &ctx.db).await?;
//         Ok(Json(media_vec))
//     }
// }
