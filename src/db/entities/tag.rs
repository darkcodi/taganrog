use chrono::{DateTime, Utc};
use crate::db::DbResult;
use crate::db::surreal_http::{SurrealDbError, SurrealHttpClient, SurrealVecDeserializable};
use crate::utils::str_utils::StringExtensions;
use crate::utils::vec_utils::RemoveExtensions;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
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
        id: &str,
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
        id: &str,
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
        let name = name.slugify();
        let query = format!("LET $tag_name = '{name}';
CREATE tag SET name = $tag_name, created_at = time::now();
SELECT * FROM tag WHERE name = $tag_name;");
        let result_vec = db.exec(query.as_str()).await?;
        let already_existed = result_vec.iter().any(|x| x.is_err());
        let mut tags_vec: Vec<Tag> = result_vec.surr_deserialize_last()?;
        let tag = tags_vec.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        let db_result = if already_existed { DbResult::Existing(tag) } else { DbResult::New(tag) };
        Ok(db_result)
    }
}
