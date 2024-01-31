use std::str::FromStr;
use axum::extract::{Extension, Path};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::api::{ApiContext, Result, ApiError};
use crate::db;
use crate::db::entities::tag::{Tag, TagId, TagWithCount};

pub fn router() -> Router {
    Router::new()
        .route("/api/tags", get(get_all_tags).post(create_tag))
        .route("/api/tags/:tag_id", get(get_tag).delete(delete_tag))
        .route("/api/tags/search", post(search_tags))
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
    let tag = db::create_tag(&ctx, &req.name).await?;
    Ok(Json(tag))
}

async fn get_all_tags(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Tag>>> {
    let tags = db::get_all_tags(&ctx).await?;
    Ok(Json(tags))
}

async fn get_tag(
    ctx: Extension<ApiContext>,
    Path(tag_id): Path<String>,
) -> Result<Json<Tag>> {
    let tag_id = TagId::from_str(&tag_id)?;
    let maybe_tag = db::get_tag_by_id(&ctx, &tag_id).await?;
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
    let maybe_tag = db::delete_tag_by_id(&ctx, &tag_id).await?;
    match maybe_tag {
        None => Err(ApiError::NotFound),
        Some(tag) => {
            Ok(Json(tag))
        },
    }
}

async fn search_tags(
    ctx: Extension<ApiContext>,
    Json(req): Json<CountMediaRequest>,
) -> Result<Json<Vec<TagWithCount>>> {
    let counts_vec = db::search_tags(&ctx, &req.tags).await?;
    Ok(Json(counts_vec))
}
