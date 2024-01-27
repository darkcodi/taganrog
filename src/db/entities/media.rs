use std::str::FromStr;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use relative_path::RelativePath;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use crate::db::entities::tag::TagId;
use crate::db::id::Id;
use crate::db::{Document, SurrealDbResult};
use crate::utils::hash_utils::MurMurHasher;
use crate::utils::str_utils::StringExtensions;

pub type MediaId = Id<"media">;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: MediaId,
    pub filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub hash: String,
    pub size: i64,
    pub location: String,
    pub was_uploaded: bool,
    pub tags: Option<Vec<String>>,
}

impl Media {
    pub fn from_file(abs_path: &std::path::Path, rel_path: &RelativePath) -> anyhow::Result<Self> {
        let id = MediaId::new();
        let filename = abs_path.file_name().ok_or(anyhow::anyhow!("filename is empty"))?.to_string_lossy().to_string();
        let location = rel_path.to_string();
        let created_at = DateTime::from_naive_utc_and_offset(chrono::Utc::now().naive_utc(), Utc);
        let file_bytes = std::fs::read(&abs_path)?;
        let content_type = infer::get(&file_bytes).map(|x| x.mime_type()).unwrap_or("application/octet-stream").to_string();
        let hash = MurMurHasher::hash_bytes(&file_bytes);
        let metadata = std::fs::metadata(&abs_path)?;
        let size = metadata.len() as i64;

        Ok(Self {
            id,
            filename,
            content_type,
            created_at,
            hash,
            size,
            location,
            was_uploaded: false,
            tags: None,
        })
    }

    pub fn contains_tag(&self, tag_name: &str) -> bool {
        if let Some(tags) = &self.tags {
            tags.contains(&tag_name.slugify())
        } else {
            false
        }
    }

    pub fn get_tags(&self) -> Vec<TagId> {
        if let Some(tags) = &self.tags {
            tags.iter().map(|x| TagId::from_str(x).unwrap()).collect()
        } else {
            vec![]
        }
    }

    pub fn get_tags_as_strings(&self) -> Vec<String> {
        if let Some(tags) = &self.tags {
            tags.iter().map(|x| x.to_string()).collect()
        } else {
            vec![]
        }
    }

    pub fn add_tag_str(&mut self, tag_name: &str) {
        if let Some(tags) = &mut self.tags {
            tags.push(tag_name.slugify());
        } else {
            self.tags = Some(vec![tag_name.slugify()]);
        }
    }

    pub fn remove_tag_str(&mut self, tag_name: &str) {
        if let Some(tags) = &mut self.tags {
            tags.retain(|x| x != &tag_name.slugify());
        }
    }

    pub async fn get_by_hash(
        hash: &str,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Option<Media>>  {
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media WHERE hash = '{hash}';");
        let maybe_media: Option<Document<Media>> = db.query(query.as_str()).await?.take(0)?;
        Ok(maybe_media.map(|x| x.into_inner()))
    }

    pub async fn create(
        media: &Media,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Media> {
        let media_id = &media.id;
        let filename = &media.filename;
        let content_type = &media.content_type;
        let created_at = &media.created_at;
        let hash = &media.hash;
        let location = &media.location;
        let size = &media.size;
        let was_uploaded = &media.was_uploaded;

        let query = format!("CREATE {media_id}
SET filename = '{filename}',
content_type = '{content_type}',
created_at = '{created_at}',
hash = '{hash}',
size = {size},
location = '{location}',
was_uploaded = {was_uploaded};");
        let maybe_media: Option<Document<Media>> = db.query(query.as_str()).await?.take(0)?;
        Ok(maybe_media.unwrap().into_inner())
    }

    pub async fn get_all(
        page_size: u64,
        page_index: u64,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Vec<Media>>  {
        let start = page_size * page_index;
        let limit = page_size;
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media ORDER BY created_at LIMIT {limit} START {start};");
        let media_vec: Vec<Document<Media>> = db.query(query.as_str()).await?.take(0)?;
        Ok(media_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    pub async fn get_by_id(
        id: &MediaId,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Option<Media>> {
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media WHERE id = '{id}';");
        let maybe_media: Option<Document<Media>> = db.query(query.as_str()).await?.take(0)?;
        Ok(maybe_media.map(|x| x.into_inner()))
    }

    pub async fn add_tag(
        media_id: &MediaId,
        tag_id: &TagId,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<()> {
        let query = format!("RELATE {media_id}->has->{tag_id} SET time.added = time::now();");
        db.query(query.as_str()).await?;
        Ok(())
    }

    pub async fn remove_tag(
        media_id: &MediaId,
        tag_name: &str,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<()> {
        let tag_name = tag_name.slugify();
        let query = format!("DELETE FROM has WHERE in = {media_id} and out.name = '{tag_name}';");
        db.query(query.as_str()).await?;
        Ok(())
    }

    pub async fn delete_by_id(
        id: &MediaId,
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Option<Media>> {
        let query = format!("DELETE FROM media WHERE id = '{id}' RETURN BEFORE;");
        let maybe_media: Option<Document<Media>> = db.query(query.as_str()).await?.take(0)?;
        Ok(maybe_media.map(|x| x.into_inner()))
    }

    pub async fn migrate(db: &Surreal<Db>) -> SurrealDbResult<()> {
        let query = "BEGIN TRANSACTION;
DEFINE TABLE media SCHEMAFULL;
DEFINE FIELD filename ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD content_type ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD created_at ON media TYPE datetime VALUE $value OR time::now();
DEFINE FIELD hash ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD size ON media TYPE int ASSERT $value != 0 VALUE $value;
DEFINE FIELD location ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD was_uploaded ON media TYPE bool ASSERT $value != NONE VALUE $value;
DEFINE INDEX media_hash ON TABLE media COLUMNS hash UNIQUE;
DEFINE INDEX media_has_tag_uc ON TABLE has COLUMNS in, out UNIQUE;
COMMIT TRANSACTION;";
        db.query(query).await?;
        Ok(())
    }

    pub async fn search(
        tags: &[String],
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Vec<Media>>  {
        let tags_arr = tags.iter().map(|x| format!("'{}'", x.slugify())).join(", ");
        let query = format!("SELECT *, ->has->tag.name AS tags
FROM media
WHERE ->has->tag.name CONTAINSALL [{tags_arr}];");
        let media_vec: Vec<Document<Media>> = db.query(query.as_str()).await?.take(0)?;
        Ok(media_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    pub async fn get_untagged(
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Vec<Media>>  {
        let query = "SELECT *, ->has->tag.name AS tags
FROM media
WHERE array::len(->has->tag) == 0;";
        let media_vec: Vec<Document<Media>> = db.query(query).await?.take(0)?;
        Ok(media_vec.into_iter().map(|x| x.into_inner()).collect())
    }
}
