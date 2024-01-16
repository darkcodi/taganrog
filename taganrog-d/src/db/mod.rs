use rusqlite::params;
use tracing::info;
use crate::db::entities::{Media, Tag};
use crate::http::ApiContext;

pub mod entities;
pub mod id;

pub struct DbRepo;

impl DbRepo {
    pub async fn insert_media(ctx: &ApiContext, media: &Media) -> anyhow::Result<()> {
        let sql = "INSERT INTO media (id, filename, relative_path, imported_at, content_type, hash, size, was_uploaded) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
        let id = media.id.to_string();
        let filename = media.filename.clone();
        let relative_path = media.relative_path.clone();
        let imported_at = media.imported_at.timestamp();
        let content_type = media.content_type.clone();
        let hash = media.hash.clone();
        let size = media.size;
        let was_uploaded = media.was_uploaded;
        info!("Executing SQL: {}", sql);

        ctx.db.call(move |conn| {
            conn.execute(sql, params![
                id,
                filename,
                relative_path,
                imported_at,
                content_type,
                hash,
                size,
                was_uploaded
            ])?;

            Ok(())
        }).await?;

        Ok(())
    }
}

pub async fn migrate(ctx: ApiContext) -> anyhow::Result<()> {
    info!("Starting DB migration...");

    ctx.db.call(|conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS media (
            id TEXT PRIMARY KEY,
            filename TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            imported_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            content_type TEXT NOT NULL,
            hash TEXT NOT NULL,
            size INTEGER NOT NULL,
            was_uploaded BOOLEAN NOT NULL DEFAULT FALSE
        )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS media_tags (
            media_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (media_id, tag_id)
        )",
            [],
        )?;

        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS ix_media_hash ON media (hash)",
            [],
        )?;

        Ok(())
    }).await?;

    info!("DB Migrated!");
    Ok(())
}
