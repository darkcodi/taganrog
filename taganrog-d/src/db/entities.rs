use chrono::{DateTime, Utc};
use relative_path::RelativePath;
use crate::db::id::Id;
use crate::utils::hash_utils::MurMurHasher;

pub type MediaId = Id<"media">;
pub type TagId = Id<"tag">;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: MediaId,
    pub filename: String,
    pub relative_path: String,
    pub imported_at: DateTime<Utc>,
    pub content_type: String,
    pub hash: String,
    pub size: i64,
    pub was_uploaded: bool,
    pub tags: Vec<TagId>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub media: Vec<MediaId>,
}

impl Media {
    pub fn from_file(abs_path: &std::path::Path, rel_path: &RelativePath) -> anyhow::Result<Self> {
        let id = MediaId::new();
        let filename = abs_path.file_name().ok_or(anyhow::anyhow!("filename is empty"))?.to_string_lossy().to_string();
        let relative_path = rel_path.to_string();
        let imported_at = DateTime::from_naive_utc_and_offset(chrono::Utc::now().naive_utc(), Utc);
        let file_bytes = std::fs::read(&abs_path)?;
        let content_type = infer::get(&file_bytes).map(|x| x.mime_type()).unwrap_or("application/octet-stream").to_string();
        let hash = MurMurHasher::hash_bytes(&file_bytes);
        let metadata = std::fs::metadata(&abs_path)?;
        let size = metadata.len() as i64;

        Ok(Self {
            id,
            filename,
            relative_path,
            imported_at,
            content_type,
            hash,
            size,
            was_uploaded: false,
            tags: vec![],
        })
    }
}
