use chrono::{DateTime, Utc};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
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
    pub tags: Vec<Tag>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct MediaPage {
    pub media_vec: Vec<Media>,
    pub page_index: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub elapsed: u64,
}
