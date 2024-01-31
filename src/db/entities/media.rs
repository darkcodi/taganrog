use std::str::FromStr;
use chrono::{DateTime, Utc};
use relative_path::RelativePath;
use crate::db::entities::tag::TagId;
use crate::db::id::Id;
use crate::utils::hash_utils::MurMurHasher;
use crate::utils::str_utils::StringExtensions;

pub type MediaId = Id<"media">;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: MediaId,
    pub filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub hash: String,
    pub size: i64,
    pub location: String,
    pub was_uploaded: bool,
    pub tags: Option<Vec<String>>,
}

impl Media {
    pub fn from_file(abs_path: &std::path::Path, rel_path: &RelativePath) -> anyhow::Result<Self> {
        let id = MediaId::new();
        let filename = abs_path.file_name().ok_or(anyhow::anyhow!("filename is empty"))?.to_string_lossy().to_string();
        let location = rel_path.to_string();
        let created_at = DateTime::from_naive_utc_and_offset(chrono::Utc::now().naive_utc(), Utc);
        let file_bytes = std::fs::read(&abs_path)?;
        let content_type = infer::get(&file_bytes).map(|x| x.mime_type()).unwrap_or("application/octet-stream").to_string();
        let hash = MurMurHasher::hash_bytes(&file_bytes);
        let metadata = std::fs::metadata(&abs_path)?;
        let size = metadata.len() as i64;

        Ok(Self {
            id,
            filename,
            content_type,
            created_at,
            hash,
            size,
            location,
            was_uploaded: false,
            tags: Some(vec![]),
        })
    }

    pub fn contains_tag(&self, tag_name: &str) -> bool {
        if let Some(tags) = &self.tags {
            tags.contains(&tag_name.slugify())
        } else {
            false
        }
    }

    pub fn get_tags(&self) -> Vec<TagId> {
        if let Some(tags) = &self.tags {
            tags.iter().map(|x| TagId::from_str(x).unwrap()).collect()
        } else {
            vec![]
        }
    }

    pub fn get_tags_as_strings(&self) -> Vec<String> {
        if let Some(tags) = &self.tags {
            tags.iter().map(|x| x.to_string()).collect()
        } else {
            vec![]
        }
    }

    pub fn add_tag_str(&mut self, tag_name: &str) {
        if let Some(tags) = &mut self.tags {
            tags.push(tag_name.slugify());
        } else {
            self.tags = Some(vec![tag_name.slugify()]);
        }
    }

    pub fn remove_tag_str(&mut self, tag_name: &str) {
        if let Some(tags) = &mut self.tags {
            tags.retain(|x| x != &tag_name.slugify());
        }
    }
}
