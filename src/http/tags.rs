use axum::extract::{Extension, Path};
use axum::routing::{get};
use axum::{Json, Router};
use chrono::NaiveDateTime;
use itertools::Itertools;
use sqlx::FromRow;

use crate::http::{ApiContext, Error, Result, ResultExt};

pub fn router() -> Router {
    Router::new()
        .route("/api/tags", get(get_all_tags).post(create_tag))
        .route("/api/tags/:tag_id", get(get_tag).delete(delete_tag))
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, FromRow)]
pub struct Tag {
    id: i64,
    name: String,
    created_at: NaiveDateTime,
}

#[derive(serde::Deserialize, Debug, Default)]
struct CreateTag {
    name: String,
}

async fn create_tag(
    ctx: Extension<ApiContext>,
    Json(req): Json<CreateTag>,
) -> Result<Json<Tag>> {
    let name = slugify(&req.name);
    let created_at = chrono::Utc::now().naive_utc();
    let id = sqlx::query_scalar(r#"insert into tags (name, created_at) values ($1, $2) returning id"#)
        .bind(&name)
        .bind(&created_at)
        .fetch_one(&ctx.db)
        .await
        .on_constraint("tags_name", |_| {
            Error::unprocessable_entity([("name", format!("duplicate tag slug: {}", &name))])
        })?;

    Ok(Json(Tag {
        id,
        name,
        created_at,
    }))
}

async fn get_tag(
    ctx: Extension<ApiContext>,
    Path(tag_id): Path<i64>,
) -> Result<Json<Tag>> {
    let tag = sqlx::query_as::<_, Tag>(r#"select id, name, created_at from tags where id = $1"#)
        .bind(tag_id)
        .fetch_one(&ctx.db)
        .await?;

    Ok(Json(tag))
}

async fn get_all_tags(
    ctx: Extension<ApiContext>,
) -> Result<Json<Vec<Tag>>> {
    let media_vec = sqlx::query_as::<_, Tag>(r#"select id, name, created_at from tags"#)
        .fetch_all(&ctx.db)
        .await?;

    Ok(Json(media_vec))
}

async fn delete_tag(
    ctx: Extension<ApiContext>,
    Path(tag_id): Path<i64>,
) -> Result<()> {
    sqlx::query_as::<_, Tag>(r#"select id, name, created_at from tags where id = $1"#)
        .bind(tag_id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    sqlx::query(r#"delete from tags where id = $1"#)
        .bind(tag_id)
        .execute(&ctx.db)
        .await?;

    Ok(())
}

/// Convert a title string to a slug for identifying an article.
///
/// E.g. `slugify("Doctests are the Bee's Knees") == "doctests-are-the-bees-knees"`
///
fn slugify(string: &str) -> String {
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
