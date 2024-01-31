use serde::{Deserialize, Serialize};
use tracing::info;
use crate::api::ApiContext;
use crate::db::entities::media::{Media, MediaId};
use crate::db::entities::tag::{Tag, TagId, TagWithCount};
use crate::db::executors::{DbExecutor, DbInsertResult};
use crate::db::executors::surreal::SurrealDbExecutor;
use crate::utils::str_utils::StringExtensions;

pub mod entities;
pub mod id;
mod executors;

#[derive(Debug, Serialize, Deserialize)]
enum DbOperation {
    CreateMedia(Media),
    CreateTag(Tag),
    DeleteMedia(MediaId),
    DeleteTag(TagId),
    AddTagToMedia(MediaId, TagId),
    RemoveTagFromMedia(MediaId, TagId),
}

pub async fn init(ctx: &ApiContext) -> anyhow::Result<()> {
    info!("Starting DB migration...");
    SurrealDbExecutor::migrate(&ctx.db).await?;
    info!("DB Migrated!");

    info!("Starting DB import from WAL...");
    let mut wal = ctx.wal.lock().await;
    let iter = wal.iter(..).unwrap();
    for item in iter {
        let operation: DbOperation = rmp_serde::from_slice(&*item?)?;
        SurrealDbExecutor::apply_db_operation(&ctx.db, operation).await?;
    }
    info!("DB Imported!");

    Ok(())
}

async fn write_wal(ctx: &ApiContext, operation: DbOperation) -> anyhow::Result<()> {
    info!("Writing to WAL: {:?}", operation);
    let mut wal = ctx.wal.lock().await;
    let mut serialized_operation = rmp_serde::to_vec(&operation)?;
    wal.write(&mut serialized_operation)?;
    wal.flush()?;
    info!("WAL written!");
    Ok(())
}

pub async fn get_media_by_id(ctx: &ApiContext, id: &MediaId) -> anyhow::Result<Option<Media>> {
    let maybe_media = SurrealDbExecutor::get_media_by_id(&ctx.db, id).await?;
    Ok(maybe_media)
}

pub async fn get_media_by_hash(ctx: &ApiContext, hash: &str) -> anyhow::Result<Option<Media>> {
    let maybe_media = SurrealDbExecutor::get_media_by_hash(&ctx.db, hash).await?;
    Ok(maybe_media)
}

pub async fn get_all_media(ctx: &ApiContext, page_size: u64, page_index: u64) -> anyhow::Result<Vec<Media>> {
    let media_vec = SurrealDbExecutor::get_all_media(&ctx.db, page_size, page_index).await?;
    Ok(media_vec)
}

pub async fn get_untagged_media(ctx: &ApiContext) -> anyhow::Result<Vec<Media>> {
    let media_vec = SurrealDbExecutor::get_untagged_media(&ctx.db).await?;
    Ok(media_vec)
}

pub async fn search_media(ctx: &ApiContext, tags: &[String], page_size: u64, page_index: u64) -> anyhow::Result<Vec<Media>> {
    let media_vec = SurrealDbExecutor::search_media(&ctx.db, tags, page_size, page_index).await?;
    Ok(media_vec)
}

pub async fn get_all_tags(ctx: &ApiContext) -> anyhow::Result<Vec<Tag>> {
    let tags = SurrealDbExecutor::get_all_tags(&ctx.db).await?;
    Ok(tags)
}

pub async fn get_tag_by_id(ctx: &ApiContext, id: &TagId) -> anyhow::Result<Option<Tag>> {
    let maybe_tag = SurrealDbExecutor::get_tag_by_id(&ctx.db, id).await?;
    Ok(maybe_tag)
}

pub async fn search_tags(ctx: &ApiContext, tags: &[String], page_size: u64, page_index: u64) -> anyhow::Result<Vec<TagWithCount>> {
    let tag_vec = SurrealDbExecutor::search_tags(&ctx.db, tags, page_size, page_index).await?;
    Ok(tag_vec)
}

pub async fn create_media(ctx: &ApiContext, media: &Media) -> anyhow::Result<Media> {
    let result = SurrealDbExecutor::create_media(&ctx.db, media).await?;
    match result {
        DbInsertResult::Existing(media) => {
            Ok(media)
        }
        DbInsertResult::New(media) => {
            let operation = DbOperation::CreateMedia(media.clone());
            write_wal(ctx, operation).await?;
            Ok(media)
        }
    }
}

pub async fn create_tag(ctx: &ApiContext, name: &str) -> anyhow::Result<Tag> {
    let maybe_tag = SurrealDbExecutor::get_tag_by_name(&ctx.db, name).await?;
    if let Some(tag) = maybe_tag {
        return Ok(tag);
    }
    let tag = Tag {
        id: TagId::new(),
        name: name.slugify(),
        created_at: chrono::Utc::now(),
    };
    let result = SurrealDbExecutor::create_tag(&ctx.db, &tag).await?;
    match result {
        DbInsertResult::Existing(tag) => {
            Ok(tag)
        }
        DbInsertResult::New(tag) => {
            let operation = DbOperation::CreateTag(tag.clone());
            write_wal(ctx, operation).await?;
            Ok(tag)
        }
    }
}

pub async fn delete_media_by_id(ctx: &ApiContext, id: &MediaId) -> anyhow::Result<Option<Media>> {
    let maybe_media = SurrealDbExecutor::delete_media_by_id(&ctx.db, id).await?;
    if let Some(media) = &maybe_media {
        let operation = DbOperation::DeleteMedia(media.id.clone());
        write_wal(ctx, operation).await?;
    }
    Ok(maybe_media)
}

pub async fn delete_tag_by_id(ctx: &ApiContext, id: &TagId) -> anyhow::Result<Option<Tag>> {
    let maybe_tag = SurrealDbExecutor::delete_tag_by_id(&ctx.db, id).await?;
    if let Some(tag) = &maybe_tag {
        let operation = DbOperation::DeleteTag(tag.id.clone());
        write_wal(ctx, operation).await?;
    }
    Ok(maybe_tag)
}

pub async fn add_tag_to_media(ctx: &ApiContext, media_id: &MediaId, tag_id: &TagId) -> anyhow::Result<()> {
    SurrealDbExecutor::add_tag_to_media(&ctx.db, media_id, tag_id).await?;
    let operation = DbOperation::AddTagToMedia(media_id.clone(), tag_id.clone());
    write_wal(ctx, operation).await?;
    Ok(())
}

pub async fn remove_tag_from_media(ctx: &ApiContext, media_id: &MediaId, tag_id: &TagId) -> anyhow::Result<()> {
    SurrealDbExecutor::remove_tag_from_media(&ctx.db, media_id, tag_id).await?;
    let operation = DbOperation::RemoveTagFromMedia(media_id.clone(), tag_id.clone());
    write_wal(ctx, operation).await?;
    Ok(())
}
