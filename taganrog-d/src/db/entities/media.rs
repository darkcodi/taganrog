use chrono::{DateTime, Utc};
use itertools::Itertools;
use crate::db::entities::tag::TagId;
use crate::db::id::Id;
use crate::db::surreal_http::{SurrealDbError, SurrealDbResult, SurrealHttpClient, SurrealVecDeserializable};
use crate::utils::str_utils::StringExtensions;
use crate::utils::vec_utils::RemoveExtensions;

pub type MediaId = Id<"media">;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: MediaId,
    pub original_filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub hash: String,
    pub size: i64,
    pub location: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MediaWithTags {
    pub id: MediaId,
    pub original_filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub hash: String,
    pub location: String,
    pub size: i64,
    pub tags: Vec<String>,
}

impl From<Media> for MediaWithTags {
    fn from(value: Media) -> Self {
        Self {
            id: value.id,
            original_filename: value.original_filename,
            content_type: value.content_type,
            created_at: value.created_at,
            hash: value.hash,
            location: value.location,
            size: value.size,
            tags: Vec::new(),
        }
    }
}

impl Media {
    pub async fn get_by_hash(
        hash: &str,
        db: &SurrealHttpClient,
    ) -> Result<Option<MediaWithTags>, SurrealDbError>  {
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media WHERE hash = '{hash}';");
        let mut media_vec: Vec<MediaWithTags> = db.exec(query.as_str())
            .await?
            .surr_deserialize_last()?;
        let maybe_media = media_vec.remove_last();
        Ok(maybe_media)
    }

    pub async fn create(
        media: &Media,
        db: &SurrealHttpClient,
    ) -> Result<MediaWithTags, SurrealDbError> {
        let media_id = &media.id;
        let original_filename = &media.original_filename;
        let content_type = &media.content_type;
        let created_at = &media.created_at;
        let hash = &media.hash;
        let location = &media.location;
        let size = &media.size;

        let query = format!("CREATE {media_id}
SET original_filename = '{original_filename}',
content_type = '{content_type}',
created_at = '{created_at}',
hash = '{hash}',
location = '{location}',
size = {size};");
        let result_vec = db.exec(query.as_str()).await?;
        let mut media_vec: Vec<Media> = result_vec.surr_deserialize_last()?;
        let media: Media = media_vec.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        let media: MediaWithTags = media.into();
        Ok(media)
    }

    pub async fn get_all(
        page_size: u64,
        page_index: u64,
        db: &SurrealHttpClient,
    ) -> Result<Vec<MediaWithTags>, SurrealDbError>  {
        let start = page_size * page_index;
        let limit = page_size;
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media ORDER BY created_at LIMIT {limit} START {start};");
        let media_vec: Vec<MediaWithTags> = db.exec(query.as_str())
            .await?
            .surr_deserialize_last()?;
        Ok(media_vec)
    }

    pub async fn get_by_id(
        id: &MediaId,
        db: &SurrealHttpClient,
    ) -> Result<Option<MediaWithTags>, SurrealDbError> {
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media WHERE id = '{id}';");
        let mut media_vec: Vec<MediaWithTags> = db.exec(query.as_str())
            .await?
            .surr_deserialize_last()?;
        let maybe_media = media_vec.remove_last();
        Ok(maybe_media)
    }

    pub async fn add_tag(
        media_id: &MediaId,
        tag_id: &TagId,
        db: &SurrealHttpClient,
    ) -> Result<(), SurrealDbError> {
        let query = format!("RELATE {media_id}->has->{tag_id} SET time.added = time::now();");
        let mut result = db.exec(query.as_str()).await?;
        let _ = result.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        Ok(())
    }

    pub async fn remove_tag(
        media_id: &MediaId,
        tag_name: &str,
        db: &SurrealHttpClient,
    ) -> Result<(), SurrealDbError> {
        let tag_name = tag_name.slugify();
        let query = format!("DELETE FROM has WHERE in = {media_id} and out.name = '{tag_name}';");
        let mut result = db.exec(query.as_str()).await?;
        let _ = result.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        Ok(())
    }

    pub async fn delete_by_id(
        id: &MediaId,
        db: &SurrealHttpClient,
    ) -> Result<Option<Media>, SurrealDbError> {
        let query = format!("DELETE FROM media WHERE id = '{id}' RETURN BEFORE;");
        let mut media_vec: Vec<Media> = db.exec(query.as_str())
            .await?
            .surr_deserialize_last()?;
        let maybe_media = media_vec.remove_last();
        Ok(maybe_media)
    }

    pub async fn migrate(db: &SurrealHttpClient) -> Result<(), SurrealDbError> {
        let query = "BEGIN TRANSACTION;
DEFINE TABLE media SCHEMAFULL;
DEFINE FIELD original_filename ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD content_type ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD created_at ON media TYPE datetime VALUE $value OR time::now();
DEFINE FIELD hash ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD location ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD size ON media TYPE int ASSERT $value != 0 VALUE $value;
DEFINE INDEX media_hash ON TABLE media COLUMNS hash UNIQUE;
DEFINE INDEX media_has_tag_uc ON TABLE has COLUMNS in, out UNIQUE;
COMMIT TRANSACTION;";
        let mut result_vec = db.exec(query).await?;
        let last_result = result_vec.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        if let SurrealDbResult::Err { detail, .. } = last_result {
            return Err(SurrealDbError::QueryExecutionError(detail));
        }
        Ok(())
    }

    pub async fn search(
        tags: &[String],
        db: &SurrealHttpClient,
    ) -> Result<Vec<MediaWithTags>, SurrealDbError>  {
        let tags_arr = tags.iter().map(|x| format!("'{}'", x.slugify())).join(", ");
        let query = format!("SELECT *, ->has->tag.name AS tags
FROM media
WHERE ->has->tag.name CONTAINSALL [{tags_arr}];");
        let media_vec: Vec<MediaWithTags> = db.exec(query.as_str())
            .await?
            .surr_deserialize_last()?;
        Ok(media_vec)
    }

    pub async fn get_untagged(
        db: &SurrealHttpClient,
    ) -> Result<Vec<MediaWithTags>, SurrealDbError>  {
        let media_vec: Vec<MediaWithTags> = db.exec("SELECT *, ->has->tag.name AS tags
FROM media
WHERE array::len(->has->tag) == 0;")
            .await?
            .surr_deserialize_last()?;
        Ok(media_vec)
    }
}
