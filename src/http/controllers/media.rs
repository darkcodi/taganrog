use std::ffi::OsStr;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_auth::AuthBearer;
use chrono::{DateTime, Utc};
use s3::{Bucket, Region};
use s3::creds::Credentials;
use uuid::Uuid;
use crate::config::S3Configuration;
use crate::db::entities::media::{Media, MediaId, MediaWithTags};
use crate::http::error::{ApiError};
use crate::http::{ApiContext, auth, Result};
use crate::http::auth::MyCustomBearerAuth;
use crate::utils::hash_utils::MurMurHasher;

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 52_428_800; // 50 MB

pub fn router() -> Router {
    Router::new()
        .route("/api/media", get(get_all_media).post(create_media))
        // .route("/api/media/:media_id", get(get_media).delete(delete_media))
        // .route("/api/media/:media_id/tag", post(add_tag_to_media).delete(delete_tag_from_media))
        // .route("/api/media/search", post(search_media))
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
    let new_filename = format!("{}{}", id.just_id(), &file.extension.clone().unwrap_or("".to_string()));
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
    let media = Media::insert(&media, &ctx.db).await?;
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

// async fn get_media(
//     ctx: Extension<ApiContext>,
//     MyCustomBearerAuth(token): MyCustomBearerAuth,
//     Path(media_id): Path<i64>,
// ) -> Result<Json<MediaResponse>> {
//     auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;
//
//     let media = media::find_by_id(media_id, &ctx.db)
//         .await?
//         .ok_or(Error::NotFound)?;
//
//     Ok(Json(media))
// }
//
// async fn delete_media(
//     ctx: Extension<ApiContext>,
//     MyCustomBearerAuth(token): MyCustomBearerAuth,
//     Path(media_id): Path<i64>,
// ) -> Result<()> {
//     auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;
//
//     let media = MediaEntity::find_by_id(media_id)
//         .one(&ctx.db)
//         .await?
//         .ok_or(Error::NotFound)?;
//
//     let bucket = get_bucket(&ctx.config.s3);
//     bucket.delete_object(&media.new_filename).await?;
//
//     media.delete(&ctx.db).await?;
//
//     Ok(())
// }
//
// async fn add_tag_to_media(
//     ctx: Extension<ApiContext>,
//     MyCustomBearerAuth(token): MyCustomBearerAuth,
//     Path(media_id): Path<i64>,
//     Json(req): Json<TagBody>,
// ) -> Result<Json<MediaResponse>> {
//     auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;
//
//     let mut media = media::find_by_id(media_id, &ctx.db)
//         .await?
//         .ok_or(Error::NotFound)?;
//
//     let tag_name = slugify(&req.name);
//     if media.tags.contains(&tag_name) {
//         return Err(Error::conflict(media));
//     }
//
//     let tag = tag::ensure_exists(tag_name.as_str(), &ctx.db).await?;
//     media_tag::ensure_exists(media_id, tag.id, &ctx.db).await?;
//     media.tags.push(tag.name);
//     Ok(Json(media))
// }
//
// async fn delete_tag_from_media(
//     ctx: Extension<ApiContext>,
//     MyCustomBearerAuth(token): MyCustomBearerAuth,
//     Path(media_id): Path<i64>,
//     Json(req): Json<TagBody>,
// ) -> Result<Json<MediaResponse>> {
//     auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;
//
//     let mut media = media::find_by_id(media_id, &ctx.db)
//         .await?
//         .ok_or(Error::NotFound)?;
//
//     let tag_name = slugify(&req.name);
//     let tag_index = media.tags.iter().position(|x| x.eq(&tag_name));
//     if tag_index.is_none() {
//         return Ok(Json(media));
//     }
//
//     let tag = tag::ensure_exists(tag_name.as_str(), &ctx.db).await?;
//     media_tag::ensure_not_exists(media_id, tag.id, &ctx.db).await?;
//     media.tags.remove(tag_index.unwrap());
//     Ok(Json(media))
// }
//
// async fn search_media(
//     ctx: Extension<ApiContext>,
//     MyCustomBearerAuth(token): MyCustomBearerAuth,
//     Json(req): Json<SearchBody>,
// ) -> Result<Json<Vec<MediaResponse>>> {
//     auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;
//
//     let media_vec: Vec<MediaResponse> = media::search(&req.tags, &ctx.db).await?;
//
//     Ok(Json(media_vec))
// }

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
