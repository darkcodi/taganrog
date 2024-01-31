pub mod surreal;

use crate::db::DbOperation;
use crate::db::entities::media::{Media, MediaId};
use crate::db::entities::tag::{Tag, TagId, TagWithCount};

#[async_trait::async_trait]
pub trait DbExecutor {
    type Db: Sized + Send + Sync;
    type DbError: Sized + Send + Sync;

    // migration
    async fn migrate(db: &Self::Db) -> Result<(), Self::DbError>;

    // read-only operations
    async fn get_media_by_id(db: &Self::Db, id: &MediaId) -> Result<Option<Media>, Self::DbError>;
    async fn get_media_by_hash(db: &Self::Db, hash: &str) -> Result<Option<Media>, Self::DbError>;
    async fn get_all_media(db: &Self::Db, page_size: u64, page_index: u64) -> Result<Vec<Media>, Self::DbError>;
    async fn get_untagged_media(db: &Self::Db) -> Result<Vec<Media>, Self::DbError>;
    async fn search_media(db: &Self::Db, tags: &[String], page_size: u64, page_index: u64) -> Result<Vec<Media>, Self::DbError>;
    async fn get_all_tags(db: &Self::Db) -> Result<Vec<Tag>, Self::DbError>;
    async fn get_tag_by_id(db: &Self::Db, id: &TagId) -> Result<Option<Tag>, Self::DbError>;
    async fn get_tag_by_name(db: &Self::Db, name: &str) -> Result<Option<Tag>, Self::DbError>;
    async fn search_tags(db: &Self::Db, tags: &[String], page_size: u64, page_index: u64) -> Result<Vec<TagWithCount>, Self::DbError>;

    // write operations
    async fn create_media(db: &Self::Db, media: &Media) -> Result<DbInsertResult<Media>, Self::DbError>;
    async fn create_tag(db: &Self::Db, name: &Tag) -> Result<DbInsertResult<Tag>, Self::DbError>;
    async fn delete_media_by_id(db: &Self::Db, id: &MediaId) -> Result<Option<Media>, Self::DbError>;
    async fn delete_tag_by_id(db: &Self::Db, id: &TagId) -> Result<Option<Tag>, Self::DbError>;
    async fn add_tag_to_media(db: &Self::Db, media_id: &MediaId, tag_id: &TagId) -> Result<(), Self::DbError>;
    async fn remove_tag_from_media(db: &Self::Db, media_id: &MediaId, tag_id: &TagId) -> Result<(), Self::DbError>;

    async fn apply_db_operation(db: &Self::Db, operation: DbOperation) -> Result<(), Self::DbError> {
        match operation {
            DbOperation::CreateMedia(media) => { Self::create_media(db, &media).await?; },
            DbOperation::CreateTag(tag) => { Self::create_tag(db, &tag).await?; },
            DbOperation::DeleteMedia(id) => { Self::delete_media_by_id(db, &id).await?; },
            DbOperation::DeleteTag(id) => { Self::delete_tag_by_id(db, &id).await?; },
            DbOperation::AddTagToMedia(media_id, tag_id) => { Self::add_tag_to_media(db, &media_id, &tag_id).await?; },
            DbOperation::RemoveTagFromMedia(media_id, tag_id) => { Self::remove_tag_from_media(db, &media_id, &tag_id).await?; },
        };
        Ok(())
    }
}

pub enum DbInsertResult<T> {
    Existing(T),
    New(T),
}

impl<T> DbInsertResult<T> {
    pub fn safe_unwrap(self) -> T {
        match self {
            DbInsertResult::Existing(x) => x,
            DbInsertResult::New(x) => x,
        }
    }
}
