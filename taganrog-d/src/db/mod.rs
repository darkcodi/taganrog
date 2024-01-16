use tracing::info;
use crate::http::ApiContext;

pub mod entities;
pub mod id;

pub enum DbResult<T> {
    Existing(T),
    New(T),
}

impl<T> DbResult<T> {
    pub fn safe_unwrap(self) -> T {
        match self {
            DbResult::Existing(x) => x,
            DbResult::New(x) => x,
        }
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
            original_filename TEXT NOT NULL,
            content_type TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            hash TEXT NOT NULL,
            size INTEGER NOT NULL,
            relative_path TEXT NOT NULL
        )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS media_tags (
            media_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
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
