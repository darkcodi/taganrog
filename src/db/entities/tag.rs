use chrono::{DateTime, Utc};
use itertools::Itertools;
use crate::db::DbResult;
use crate::db::id::Id;
use crate::db::surreal_http::{SurrealDbError, SurrealDbResult, SurrealHttpClient, SurrealVecDeserializable};
use crate::utils::str_utils::StringExtensions;
use crate::utils::vec_utils::RemoveExtensions;

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
        db: &SurrealHttpClient,
    ) -> Result<Vec<Tag>, SurrealDbError> {
        let result: Vec<Tag> = db.exec("SELECT * FROM tag;")
            .await?
            .surr_deserialize_last()?;
        Ok(result)
    }

    pub async fn get_by_id(
        id: &TagId,
        db: &SurrealHttpClient,
    ) -> Result<Option<Tag>, SurrealDbError> {
        let query = format!("SELECT * FROM tag WHERE id = '{id}';");
        let mut tags_vec: Vec<Tag> = db.exec(query.as_str())
            .await?
            .surr_deserialize_last()?;
        let maybe_tag = tags_vec.remove_last();
        Ok(maybe_tag)
    }

    pub async fn delete_by_id(
        id: &TagId,
        db: &SurrealHttpClient,
    ) -> Result<Option<Tag>, SurrealDbError> {
        let query = format!("DELETE FROM tag WHERE id = '{id}' RETURN BEFORE;");
        let mut tags_vec: Vec<Tag> = db.exec(query.as_str())
            .await?
            .surr_deserialize_last()?;
        let maybe_tag = tags_vec.remove_last();
        Ok(maybe_tag)
    }

    pub async fn ensure_exists(
        name: &str,
        db: &SurrealHttpClient,
    ) -> Result<DbResult<Tag>, SurrealDbError> {
        let tag_id = TagId::new();
        let name = name.slugify();
        let query = format!("LET $tag_name = '{name}';
CREATE {tag_id} SET name = $tag_name, created_at = time::now();
SELECT * FROM tag WHERE name = $tag_name;");
        let result_vec = db.exec(query.as_str()).await?;
        let already_existed = result_vec.iter().any(|x| x.is_err());
        let mut tags_vec: Vec<Tag> = result_vec.surr_deserialize_last()?;
        let tag = tags_vec.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        let db_result = if already_existed { DbResult::Existing(tag) } else { DbResult::New(tag) };
        Ok(db_result)
    }

    pub async fn count_media(
        tags: &[String],
        db: &SurrealHttpClient,
    ) -> Result<Vec<TagWithCount>, SurrealDbError> {
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
        let result: Vec<TagWithCount> = db.exec(&query)
            .await?
            .surr_deserialize_last()?;
        Ok(result)
    }

    pub async fn migrate(db: &SurrealHttpClient) -> Result<(), SurrealDbError> {
        let query = "BEGIN TRANSACTION;
DEFINE TABLE tag SCHEMAFULL;
DEFINE FIELD name ON tag TYPE string ASSERT $value != NONE VALUE $value;
DEFINE INDEX tag_name ON TABLE tag COLUMNS name UNIQUE;
DEFINE FIELD created_at ON tag TYPE datetime VALUE $value OR time::now();
COMMIT TRANSACTION;";
        let mut result_vec = db.exec(query).await?;
        let last_result = result_vec.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        if let SurrealDbResult::Err { detail, .. } = last_result {
            return Err(SurrealDbError::QueryExecutionError(detail));
        }
        Ok(())
    }
}
