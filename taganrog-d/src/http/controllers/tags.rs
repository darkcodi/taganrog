use std::str::FromStr;
use axum::extract::{Extension, Path};
use axum::routing::{get, post};
use axum::{Json, Router};
use crate::db::DbResult;
use crate::db::entities::tag::{TagWithCount, Tag, TagId};

use crate::http::{ApiContext, ApiError, Result};

pub fn router() -> Router {
    Router::new()
        .route("/api/tags", get(get_all_tags).post(create_tag))
        .route("/api/tags/:tag_id", get(get_tag).delete(delete_tag))
        .route("/api/tags/count_media", post(count_media))
}

#[derive(serde::Deserialize, Debug, Default)]
struct CreateTag {
    name: String,
}

#[derive(serde::Deserialize, Debug, Default)]
struct CountMediaRequest {
    tags: Vec<String>,
}

async fn create_tag(
    ctx: Extension<ApiContext>,
    Json(req): Json<CreateTag>,
) -> Result<Json<Tag>> {
    let db_result = Tag::ensure_exists(&req.name, &ctx.db).await?;
    match db_result {
        DbResult::Existing(tag) => Err(ApiError::Conflict {
            serialized_entity: serde_json::to_string(&tag).unwrap(),
        }),
        DbResult::New(tag) => Ok(Json(tag)),
    }
}

async fn get_all_tags(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Tag>>> {
    let tags = Tag::get_all(&ctx.db).await?;
    Ok(Json(tags))
}

async fn get_tag(
    ctx: Extension<ApiContext>,
    Path(tag_id): Path<String>,
) -> Result<Json<Tag>> {
    let tag_id = TagId::from_str(&tag_id)?;
    let maybe_tag = Tag::get_by_id(&tag_id, &ctx.db).await?;
    if maybe_tag.is_none() {
        return Err(ApiError::NotFound);
    }

    let tag = maybe_tag.unwrap();
    Ok(Json(tag))
}

async fn delete_tag(
    ctx: Extension<ApiContext>,
    Path(tag_id): Path<String>,
) -> Result<Json<Tag>> {
    let tag_id = TagId::from_str(&tag_id)?;
    let maybe_tag = Tag::delete_by_id(&tag_id, &ctx.db).await?;
    match maybe_tag {
        None => Err(ApiError::NotFound),
        Some(tag) => Ok(Json(tag)),
    }
}

async fn count_media(
    ctx: Extension<ApiContext>,
    Json(req): Json<CountMediaRequest>,
) -> Result<Json<Vec<TagWithCount>>> {
    let counts_vec = Tag::count_media(&req.tags, &ctx.db).await?;
    Ok(Json(counts_vec))
}
