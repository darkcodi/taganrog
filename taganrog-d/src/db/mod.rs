use std::str::FromStr;
use rusqlite::params;
use tracing::info;
use crate::db::entities::{Media, MediaId, MediaWithTags, Tag};
use crate::http::ApiContext;

pub mod entities;
pub mod id;

pub struct DbRepo;

impl DbRepo {
    pub async fn get_all_media_with_tags(ctx: &ApiContext, page_size: u64, page_index: u64) -> anyhow::Result<Vec<entities::MediaWithTags>> {
        let sql = "SELECT m.id, m.filename, m.relative_path, m.imported_at, m.content_type, m.hash, m.size, m.was_uploaded, t.id, t.name, t.created_at \
        FROM media m \
        LEFT JOIN media_tags mt ON m.id = mt.media_id \
        LEFT JOIN tags t ON mt.tag_id = t.id \
        WHERE m.id IN (SELECT id FROM media ORDER BY imported_at DESC LIMIT ? OFFSET ?) \
        ORDER BY m.imported_at DESC";
        info!("Executing SQL: {}", sql);

        let media_vec = ctx.db.call(move |conn| {
            let mut stmt = conn.prepare(sql)?;
            let mut rows = stmt.query(params![page_size, page_index * page_size])?;

            let mut media_vec = Vec::new();
            let mut current_media_id: Option<String> = None;
            let mut current_media: Option<Media> = None;
            let mut current_tags: Vec<Tag> = Vec::new();

            while let Some(row) = rows.next()? {
                let media_id: String = row.get(0)?;
                let filename: String = row.get(1)?;
                let relative_path: String = row.get(2)?;
                let imported_at: i64 = row.get(3)?;
                let content_type: String = row.get(4)?;
                let hash: String = row.get(5)?;
                let size: i64 = row.get(6)?;
                let was_uploaded: bool = row.get(7)?;
                let tag_id: Option<String> = row.get(8)?;
                let tag_name: Option<String> = row.get(9)?;
                let tag_created_at: Option<i64> = row.get(10)?;

                if current_media_id.is_none() || current_media_id.as_ref().unwrap() != &media_id {
                    if current_media.is_some() {
                        let media = current_media.unwrap();
                        let media_with_tags = entities::MediaWithTags {
                            media,
                            tags: current_tags,
                        };
                        media_vec.push(media_with_tags);
                    }

                    current_media_id = Some(media_id.clone());
                    current_media = Some(Media {
                        id: MediaId::from_str(media_id.as_str()).unwrap(),
                        filename,
                        relative_path,
                        imported_at: chrono::DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(imported_at, 0), chrono::Utc),
                        content_type,
                        hash,
                        size,
                        was_uploaded,
                    });
                    current_tags = Vec::new();
                }

                if tag_id.is_some() {
                    let tag_id = tag_id.unwrap();
                    let tag_name = tag_name.unwrap();
                    let tag_created_at = tag_created_at.unwrap();
                    let tag = Tag {
                        id: entities::TagId::from_str(tag_id.as_str()).unwrap(),
                        name: tag_name,
                        created_at: chrono::DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(tag_created_at, 0), chrono::Utc),
                    };
                    current_tags.push(tag);
                }
            }

            if current_media.is_some() {
                let media = current_media.unwrap();
                let media_with_tags = entities::MediaWithTags {
                    media,
                    tags: current_tags,
                };
                media_vec.push(media_with_tags);
            }

            Ok(media_vec)
        }).await?;

        Ok(media_vec)
    }

    pub async fn get_media_by_id(ctx: &ApiContext, id: MediaId) -> anyhow::Result<Option<MediaWithTags>> {
        let sql = "SELECT m.id, m.filename, m.relative_path, m.imported_at, m.content_type, m.hash, m.size, m.was_uploaded, t.id, t.name, t.created_at \
        FROM media m \
        LEFT JOIN media_tags mt ON m.id = mt.media_id \
        LEFT JOIN tags t ON mt.tag_id = t.id \
        WHERE m.id = ? \
        ORDER BY m.imported_at DESC";
        info!("Executing SQL: {}", sql);

        let media = ctx.db.call(move |conn| {
            let mut stmt = conn.prepare(sql)?;
            let mut rows = stmt.query(params![id.to_string()])?;

            let mut current_media: Option<Media> = None;
            let mut current_tags: Vec<Tag> = Vec::new();

            while let Some(row) = rows.next()? {
                let media_id: String = row.get(0)?;
                let filename: String = row.get(1)?;
                let relative_path: String = row.get(2)?;
                let imported_at: i64 = row.get(3)?;
                let content_type: String = row.get(4)?;
                let hash: String = row.get(5)?;
                let size: i64 = row.get(6)?;
                let was_uploaded: bool = row.get(7)?;
                let tag_id: Option<String> = row.get(8)?;
                let tag_name: Option<String> = row.get(9)?;
                let tag_created_at: Option<i64> = row.get(10)?;

                if current_media.is_none() {
                    current_media = Some(Media {
                        id: MediaId::from_str(media_id.as_str()).unwrap(),
                        filename,
                        relative_path,
                        imported_at: chrono::DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(imported_at, 0), chrono::Utc),
                        content_type,
                        hash,
                        size,
                        was_uploaded,
                    });
                }

                if tag_id.is_some() {
                    let tag_id = tag_id.unwrap();
                    let tag_name = tag_name.unwrap();
                    let tag_created_at = tag_created_at.unwrap();
                    let tag = Tag {
                        id: entities::TagId::from_str(tag_id.as_str()).unwrap(),
                        name: tag_name,
                        created_at: chrono::DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(tag_created_at, 0), chrono::Utc),
                    };
                    current_tags.push(tag);
                }
            }

            if current_media.is_some() {
                let media = current_media.unwrap();
                let media_with_tags = entities::MediaWithTags {
                    media,
                    tags: current_tags,
                };
                return Ok(Some(media_with_tags));
            }

            Ok(None)
        }).await?;

        Ok(media)
    }

    pub async fn get_media_by_hash(ctx: &ApiContext, hash: String) -> anyhow::Result<Option<Media>> {
        let sql = "SELECT id, filename, relative_path, imported_at, content_type, hash, size, was_uploaded FROM media WHERE hash = ?";
        info!("Executing SQL: {}", sql);

        let media = ctx.db.call(move |conn| {
            let mut stmt = conn.prepare(sql)?;
            let mut rows = stmt.query(params![hash])?;

            let row = rows.next()?;
            if row.is_none() {
                return Ok(None);
            }

            let row = row.unwrap();
            let id: String = row.get(0)?;
            let filename: String = row.get(1)?;
            let relative_path: String = row.get(2)?;
            let imported_at: i64 = row.get(3)?;
            let content_type: String = row.get(4)?;
            let hash: String = row.get(5)?;
            let size: i64 = row.get(6)?;
            let was_uploaded: bool = row.get(7)?;

            let media = Media {
                id: MediaId::from_str(id.as_str()).unwrap(),
                filename,
                relative_path,
                imported_at: chrono::DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(imported_at, 0), chrono::Utc),
                content_type,
                hash,
                size,
                was_uploaded,
            };

            Ok(Some(media))
        }).await?;

        Ok(media)
    }

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

    pub async fn delete_media(ctx: &ApiContext, id: MediaId) -> anyhow::Result<()> {
        let sql = "DELETE FROM media WHERE id = ?";
        info!("Executing SQL: {}", sql);

        ctx.db.call(move |conn| {
            conn.execute(sql, params![id.to_string()])?;

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
