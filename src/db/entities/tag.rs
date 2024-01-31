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

impl Tag {
    pub fn split_query(query: &str) -> Vec<String> {
        let mut tags = query.split(" ").map(|x| x.trim()).filter(|x| !x.is_empty()).map(|x| x.to_string()).collect::<Vec<String>>();
        if tags.len() > 0 && query.ends_with(" ") {
            tags.push("".to_string());
        }
        tags
    }
}
