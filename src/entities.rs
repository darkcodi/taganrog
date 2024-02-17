use chrono::{DateTime, Utc};
use indicium::simple::Indexable;

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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TagsAutocomplete {
    pub head: Vec<String>,
    pub last: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub size: i64,
    pub location: String,
    pub was_uploaded: bool,
    pub tags: Vec<String>,
}

impl Indexable for Media {
    fn strings(&self) -> Vec<String> {
        self.tags.clone()
    }
}
