use axum::extract::{Extension, Path};
use axum::routing::{get};
use axum::{Json, Router};
use itertools::Itertools;
use sea_orm::entity::prelude::*;
use sea_orm::Set;
use crate::entities::*;

use crate::http::{ApiContext, auth, Error, Result};
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
    auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;

    let name = slugify(&req.name);
    let existing_tag = TagEntity::find()
        .filter(TagColumn::Name.eq(&name))
        .one(&ctx.db)
        .await?;
    if existing_tag.is_some() {
        return Err(Error::conflict(existing_tag.unwrap()));
    }

    let created_at = chrono::Utc::now().naive_utc();
    let tag = ActiveTag {
        name: Set(name),
        created_at: Set(created_at),
        ..Default::default()
    };
    let tag = tag.insert(&ctx.db).await?;

    Ok(Json(tag))
}

async fn get_tag(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(tag_id): Path<i64>,
) -> Result<Json<Tag>> {
    auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;

    let tag = TagEntity::find_by_id(tag_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    Ok(Json(tag))
}

async fn get_all_tags(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
) -> Result<Json<Vec<Tag>>> {
    auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;

    let tag_vec: Vec<Tag> = TagEntity::find()
        .all(&ctx.db)
        .await?;

    Ok(Json(tag_vec))
}

async fn delete_tag(
    ctx: Extension<ApiContext>,
    MyCustomBearerAuth(token): MyCustomBearerAuth,
    Path(tag_id): Path<i64>,
) -> Result<()> {
    auth::is_token_valid(token.as_str(), ctx.config.api.bearer_token.as_str())?;

    let tag = TagEntity::find_by_id(tag_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    tag.delete(&ctx.db).await?;

    Ok(())
}

/// Convert a title string to a slug for identifying an article.
///
/// E.g. `slugify("Doctests are the Bee's Knees") == "doctests-are-the-bees-knees"`
///
pub fn slugify(string: &str) -> String {
    const QUOTE_CHARS: &[char] = &['\'', '"'];

    string
        .split(|c: char| !(QUOTE_CHARS.contains(&c) || c.is_alphanumeric()))
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut s = s.replace(QUOTE_CHARS, "");
            s.make_ascii_lowercase();
            s
        })
        .join("-")
}

#[test]
fn test_slugify() {
    assert_eq!(
        slugify("Segfaults and You: When Raw Pointers Go Wrong"),
        "segfaults-and-you-when-raw-pointers-go-wrong"
    );

    assert_eq!(
        slugify("Why are DB Admins Always Shouting?"),
        "why-are-db-admins-always-shouting"
    );

    assert_eq!(
        slugify("Converting to Rust from C: It's as Easy as 1, 2, 3!"),
        "converting-to-rust-from-c-its-as-easy-as-1-2-3"
    )
}
