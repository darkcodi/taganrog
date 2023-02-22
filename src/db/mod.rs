use tracing::info;
use crate::db::entities::media::Media;
use crate::db::entities::tag::Tag;
use crate::http::ApiContext;

pub mod surreal_http;
pub mod entities;
pub mod id;

pub enum DbResult<T> {
    Existing(T),
    New(T),
}

pub async fn migrate(ctx: ApiContext) -> anyhow::Result<()> {
    info!("Starting DB migration...");
    Tag::migrate(&ctx.db).await?;
    Media::migrate(&ctx.db).await?;
    info!("DB Migrated!");
    Ok(())
}
