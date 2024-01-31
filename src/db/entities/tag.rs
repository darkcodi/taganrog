use chrono::{DateTime, Utc};
use crate::db::id::Id;

pub type TagId = Id<"tag">;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct TagWithCount {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub count: i64,
}
