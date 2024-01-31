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
struct SearchTagsBody {
    q: String,
    p: Option<u64>,
    s: Option<u64>,
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
    Json(req): Json<SearchTagsBody>,
) -> Result<Json<Vec<TagWithCount>>> {
    let page_size = req.s.unwrap_or(10).clamp(1, 50);
    let page_index = req.p.unwrap_or(0);
    let tags = Tag::split_query(&req.q);
    if tags.is_empty() {
        return Ok(Json(vec![]));
    }
    let counts_vec = db::search_tags(&ctx, &tags, page_size, page_index).await?;
    Ok(Json(counts_vec))
}
