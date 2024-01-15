use chrono::{DateTime, Utc};
use crate::db::id::Id;

pub type MediaId = Id<"media">;
pub type TagId = Id<"tag">;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: MediaId,
    pub original_filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub hash: String,
    pub size: i64,
    pub relative_path: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TagWithCount {
    pub tag: Tag,
    pub count: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MediaWithTags {
    pub media: Media,
    pub tags: Vec<Tag>,
}
