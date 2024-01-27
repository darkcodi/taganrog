use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use surrealdb::sql::{Id, Thing};
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

#[derive(Debug, Serialize)]
pub struct Document<T>
    where
        T: DeserializeOwned,
{
    #[serde(flatten)]
    pub inner: T,
    pub id: Thing,
}

impl<T> Document<T>
    where
        T: DeserializeOwned,
{
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Document<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Outer {
            id: Thing,
            #[serde(flatten)]
            inner: serde_json::Value,
        }

        let mut helper = Outer::deserialize(deserializer)?;
        let id = match &helper.id.id {
            Id::String(id_str) => id_str.to_string(),
            Id::Number(id_num) => id_num.to_string(),
            Id::Array(id_arr) => id_arr.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",").to_string(),
            Id::Object(id_obj) => serde_json::to_string(id_obj).unwrap(),
            Id::Generate(id_gen) => "GEN".to_string(),
        };

        helper
            .inner
            .as_object_mut()
            .unwrap()
            .insert("id".to_string(), serde_json::to_value(id).unwrap());

        let inner: T = serde_json::from_value(helper.inner).unwrap();

        Ok(Document {
            inner,
            id: helper.id,
        })
    }
}
