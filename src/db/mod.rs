use tracing::info;
use crate::api::ApiContext;
use crate::db::entities::media::Media;
use crate::db::entities::tag::Tag;

pub mod entities;
pub mod id;

pub type SurrealDbError = surrealdb::Error;
pub type SurrealDbResult<T> = Result<T, SurrealDbError>;

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
    Tag::migrate(&ctx.db).await?;
    Media::migrate(&ctx.db).await?;
    info!("DB Migrated!");
    Ok(())
}
