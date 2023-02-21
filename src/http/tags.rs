use axum::extract::{Extension, Path};
use axum::routing::{get};
use axum::{Json, Router};
use crate::db::DbResult;
use crate::db::tag::Tag;

use crate::http::{ApiContext, auth, ApiError, Result};
use crate::http::auth::MyCustomBearerAuth;

pub fn router() -> Router {
    Router::new()
        .route("/api/tags", get(get_all_tags).post(create_tag))
        .route("/api/tags/:tag_id", get(get_tag).delete(delete_tag))
}

#[derive(serde::Deserialize, Debug, Default)]
struct CreateTag {
    name: String,
}

async fn create_tag(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Json(req): Json<CreateTag>,
) -> Result<Json<Tag>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

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
    MyCustomBearerAuth(token): MyCustomBearerAuth,
) -> Result<Json<Vec<Tag>>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    let tags = Tag::get_all(&ctx.db).await?;
    Ok(Json(tags))
}

async fn get_tag(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(tag_id): Path<String>,
) -> Result<Json<Tag>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    if !tag_id.starts_with("tag:") {
        return Err(ApiError::unprocessable_entity([("id", "tag id should start with 'tag:'")]));
    }
    let maybe_tag = Tag::get_by_id(tag_id.as_str(), &ctx.db).await?;
    if maybe_tag.is_none() {
        return Err(ApiError::NotFound);
    }

    let tag = maybe_tag.unwrap();
    Ok(Json(tag))
}

async fn delete_tag(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(tag_id): Path<String>,
) -> Result<Json<Tag>> {
    auth::is_token_valid(token.as_str(), ctx.cfg.api.bearer_token.as_str())?;

    if !tag_id.starts_with("tag:") {
        return Err(ApiError::unprocessable_entity([("id", "tag id should start with 'tag:'")]));
    }

    let maybe_tag = Tag::delete_by_id(tag_id.as_str(), &ctx.db).await?;
    match maybe_tag {
        None => Err(ApiError::NotFound),
        Some(tag) => Ok(Json(tag)),
    }
}
