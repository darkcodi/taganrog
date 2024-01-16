use std::ffi::OsStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use path_absolutize::Absolutize;
use relative_path::PathExt;
use tracing::warn;
use crate::db;
use crate::db::entities::Media;
use crate::http::error::{ApiError};
use crate::http::{ApiContext, Result};
use crate::utils::hash_utils::MurMurHasher;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", post(create_media))
        // .route("/api/media", get(get_all_media).post(create_media))
        // .route("/api/media/:media_id", get(get_media).delete(delete_media))
        // .route("/api/media/:media_id/add-tag", post(add_tag_to_media))
        // .route("/api/media/:media_id/remove-tag", post(delete_tag_from_media))
        // .route("/api/media/search", post(search_media))
        // .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES))
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

#[derive(serde::Deserialize, Debug, Default)]
struct Pagination {
    page_size: Option<u64>,
    page_index: Option<u64>,
}

async fn create_media(
    ctx: Extension<ApiContext>,
    Json(req): Json<ImportMediaRequest>,
) -> Result<Json<Media>> {
    let filepath = std::path::Path::new(req.filename.as_str()).absolutize_from(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 1")]))?;
    warn!("filepath: {:?}", filepath);
    warn!("workdir: {:?}", ctx.cfg.workdir);
    let relative_path = filepath.relative_to(&ctx.cfg.workdir)
        .map_err(|_| ApiError::unprocessable_entity([("filename", "invalid path 2")]))?;

    if relative_path.starts_with("..") { return Err(ApiError::unprocessable_entity([("filename", "workdir violation")])); }
    if !filepath.exists() { return Err(ApiError::unprocessable_entity([("filename", "file does not exist")])); }
    if filepath.is_dir() { return Err(ApiError::unprocessable_entity([("filename", "file is a directory")])); }

    let media = Media::from_file(&filepath, &relative_path)?;
    let existing_media = db::DbRepo::get_media_by_hash(&ctx, media.hash.clone()).await?;
    if existing_media.is_some() {
        return Err(ApiError::conflict(existing_media.unwrap()));
    }

    db::DbRepo::insert_media(&ctx, &media).await?;
    Ok(Json(media))
}

// async fn upload_media(
//     ctx: Extension<ApiContext>,
//     files: Multipart,
// ) -> Result<Json<MediaWithTags>> {
//
//     let file = File::try_from(files).await?;
//     let existing_media = Media::get_by_hash(&file.hash, &ctx.db).await?;
//     if existing_media.is_some() {
//         return Err(ApiError::conflict(existing_media.unwrap()));
//     }
//
//     let id = MediaId::new();
//     let created_at = DateTime::from_naive_utc_and_offset(Utc::now().naive_utc(), Utc);
//
//     let media = Media {
//         id: id,
//         original_filename: file.original_filename.clone(),
//         content_type: file.content_type,
//         created_at,
//         hash: file.hash,
//         size: file.size as i64,
//         location: file.original_filename,
//     };
//     let media = Media::create(&media, &ctx.db).await?;
//     Ok(Json(media))
// }
//
//
// async fn get_all_media(
//     ctx: Extension<ApiContext>,
//     Query(pagination): Query<Pagination>,
// ) -> Result<Json<Vec<MediaWithTags>>> {
//     let page_size = pagination.page_size.unwrap_or(10).clamp(1, 50);
//     let page_index = pagination.page_index.unwrap_or(0);
//     let media_vec: Vec<MediaWithTags> = Media::get_all(page_size, page_index, &ctx.db).await?;
//
//     Ok(Json(media_vec))
// }
//
// async fn get_media(
//     ctx: Extension<ApiContext>,
//     Path(media_id): Path<String>,
// ) -> Result<Json<MediaWithTags>> {
//     let media_id = MediaId::from_str(&media_id)?;
//     let maybe_media = Media::get_by_id(&media_id, &ctx.db).await?;
//     if maybe_media.is_none() {
//         return Err(ApiError::NotFound);
//     }
//
//     let media = maybe_media.unwrap();
//     Ok(Json(media))
// }
//
// async fn delete_media(
//     ctx: Extension<ApiContext>,
//     Path(media_id): Path<String>,
// ) -> Result<Json<Media>> {
//     let media_id = MediaId::from_str(&media_id)?;
//     let maybe_media = Media::delete_by_id(&media_id, &ctx.db).await?;
//     if maybe_media.is_none() {
//         return Err(ApiError::NotFound);
//     }
//
//     let media = maybe_media.unwrap();
//
//     Ok(Json(media))
// }
//
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
