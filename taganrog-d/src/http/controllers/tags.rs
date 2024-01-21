use std::str::FromStr;
use axum::extract::{Extension, Path};
use axum::routing::{get, post};
use axum::{Json, Router};
use crate::db::entities::{Tag, TagId};
use crate::db::Inserted;

use crate::http::{ApiContext, ApiError, Result};

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

#[derive(serde::Deserialize, Debug)]
struct TagSearch {
    q: String,
}

async fn get_all_tags(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Tag>>> {
    let tags = ctx.db.get_all_tags()?;
    Ok(Json(tags))
}

async fn create_tag(
    ctx: Extension<ApiContext>,
    Json(req): Json<CreateTag>,
) -> Result<Json<Tag>> {
    let tag = ctx.db.get_or_insert_tag_by_name(req.name)?;
    match tag {
        Inserted::New(tag) => Ok(Json(tag)),
        Inserted::AlreadyExists(tag) => Err(ApiError::conflict(tag)),
    }
}

async fn get_tag(
    ctx: Extension<ApiContext>,
    Path(tag_id): Path<String>,
) -> Result<Json<Tag>> {
    let tag_id = TagId::from_str(tag_id.as_str())
        .map_err(|_| ApiError::unprocessable_entity([("tag_id", "invalid id")]))?;
    let tag = ctx.db.get_tag_by_id(tag_id)?;
    if tag.is_none() {
        return Err(ApiError::NotFound);
    }
    Ok(Json(tag.unwrap()))
}

async fn delete_tag(
    ctx: Extension<ApiContext>,
    Path(tag_id): Path<String>,
) -> Result<Json<Tag>> {
    let tag_id = TagId::from_str(tag_id.as_str())
        .map_err(|_| ApiError::unprocessable_entity([("tag_id", "invalid id")]))?;
    let maybe_tag = ctx.db.get_tag_by_id(tag_id)?;
    if maybe_tag.is_none() {
        return Err(ApiError::NotFound);
    }

    let tag = maybe_tag.unwrap();
    ctx.db.delete_tag(&tag)?;
    Ok(Json(tag))
}

async fn search_tags(
    ctx: Extension<ApiContext>,
    Json(req): Json<TagSearch>,
) -> Result<Json<Vec<Tag>>> {
    if req.q.is_empty() {
        return Ok(Json(vec![]));
    }
    let mut tags = ctx.db.search_tag_by_name(req.q.clone())?;
    if req.q.len() <= 2 {
        tags = tags.into_iter().filter(|(_, distance)| *distance <= 2).collect();
    }
    let tags = tags.into_iter().take(10).map(|(tag, _)| tag).collect();
    Ok(Json(tags))
}
