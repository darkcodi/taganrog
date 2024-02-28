use chrono::{DateTime, Utc};

pub enum InsertResult<T> {
    Existing(T),
    New(T),
}

impl<T> InsertResult<T> {
    pub fn safe_unwrap(self) -> T {
        match self {
            InsertResult::Existing(x) => x,
            InsertResult::New(x) => x,
        }
    }
}

pub type MediaId = String;
pub type Tag = String;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TagsAutocomplete {
    pub head: Vec<Tag>,
    pub last: Tag,
    pub media_count: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: MediaId,
    pub filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub size: i64,
    pub location: String,
    pub was_uploaded: bool,
    pub tags: Vec<Tag>,
}
