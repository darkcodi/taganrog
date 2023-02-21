use axum::extract::{Extension, Path};
use axum::routing::{get};
use axum::{Json, Router};
use crate::db::DbResult;
use crate::db::tag::Tag;

use crate::http::{ApiContext, auth, ApiError, Result};
use crate::http::auth::MyCustomBearerAuth;
use crate::utils::str_utils::StringExtensions;

pub fn router() -> Router {
    Router::new()
        .route("/api/tags", get(get_all_tags).post(create_tag))
        //.route("/api/tags/:tag_id", get(get_tag).delete(delete_tag))
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

// async fn get_tag(
//     ctx: Extension<ApiContext>,
//     MyCustomBearerAuth(token): MyCustomBearerAuth,
//     Path(tag_id): Path<i64>,
// ) -> Result<Json<Tag>> {
//     auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;
//
//     let tag = TagEntity::find_by_id(tag_id)
//         .one(&ctx.db)
//         .await?
//         .ok_or(Error::NotFound)?;
//
//     Ok(Json(tag))
// }
//
// async fn delete_tag(
//     ctx: Extension<ApiContext>,
//     MyCustomBearerAuth(token): MyCustomBearerAuth,
//     Path(tag_id): Path<i64>,
// ) -> Result<()> {
//     auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;
//
//     let tag = TagEntity::find_by_id(tag_id)
//         .one(&ctx.db)
//         .await?
//         .ok_or(Error::NotFound)?;
//
//     tag.delete(&ctx.db).await?;
//
//     Ok(())
// }
