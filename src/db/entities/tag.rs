use chrono::{DateTime, Utc};
use itertools::Itertools;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use crate::db::{DbResult, Document, SurrealDbResult};
use crate::db::id::Id;
use crate::utils::str_utils::StringExtensions;

pub type TagId = Id<"tag">;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct TagWithCount {
    id: String,
    name: String,
    created_at: DateTime<Utc>,
    count: i64,
}

impl Tag {
    pub async fn get_all(
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Vec<Tag>> {
        let tag_vec: Vec<Document<Tag>> = db.query("SELECT * FROM tag;").await?.take(0)?;
        Ok(tag_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    pub async fn get_by_id(
        id: &TagId,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Option<Tag>> {
        let query = format!("SELECT * FROM tag WHERE id = '{id}';");
        let maybe_tag: Option<Document<Tag>> = db.query(query.as_str()).await?.take(0)?;
        Ok(maybe_tag.map(|x| x.into_inner()))
    }

    pub async fn delete_by_id(
        id: &TagId,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Option<Tag>> {
        let query = format!("DELETE FROM tag WHERE id = '{id}' RETURN BEFORE;");
        let maybe_tag: Option<Document<Tag>> = db.query(query.as_str()).await?.take(0)?;
        Ok(maybe_tag.map(|x| x.into_inner()))
    }

    pub async fn ensure_exists(
        name: &str,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<DbResult<Tag>> {
        let tag_id = TagId::new();
        let name = name.slugify();
        let query = format!("LET $tag_name = '{name}';
CREATE {tag_id} SET name = $tag_name, created_at = time::now();
SELECT * FROM tag WHERE name = $tag_name;");
        let maybe_tag: Option<Document<Tag>> = db.query(query.as_str()).await?.take(0)?;
        Ok(DbResult::New(maybe_tag.unwrap().into_inner()))
    }

    pub async fn count_media(
        tags: &[String],
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Vec<TagWithCount>> {
        let tags_arr = tags.iter().map(|x| format!("'{}'", x.slugify())).join(", ");
        let query = format!("LET $input = [{tags_arr}];
SELECT id, name, created_at, count(id)
FROM array::flatten((
    SELECT ->has->tag AS rel_tags
    FROM media
    WHERE ->has->tag.name
    CONTAINSALL $input
))
WHERE $input CONTAINSNOT name
GROUP BY id, name, created_at;");
        let tag_vec: Vec<Document<TagWithCount>> = db.query(&query).await?.take(0)?;
        Ok(tag_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    pub async fn migrate(db: &Surreal<Db>) -> SurrealDbResult<()> {
        let query = "BEGIN TRANSACTION;
DEFINE TABLE tag SCHEMAFULL;
DEFINE FIELD name ON tag TYPE string ASSERT $value != NONE VALUE $value;
DEFINE INDEX tag_name ON TABLE tag COLUMNS name UNIQUE;
DEFINE FIELD created_at ON tag TYPE datetime VALUE $value OR time::now();
COMMIT TRANSACTION;";
        db.query(query).await?;
        Ok(())
    }
}
