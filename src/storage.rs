use std::path::{PathBuf};
use serde::{Deserialize, Serialize};
use crate::config::ConfigError;
use crate::entities::{Media, MediaId, Tag};
use crate::error::TaganrogError;

#[derive(Debug, Serialize, Deserialize)]
pub enum DbOperation {
    CreateMedia { media: Media },
    DeleteMedia { media_id: MediaId },
    AddTag { media_id: MediaId, tag: Tag },
    RemoveTag { media_id: MediaId, tag: Tag },
}

pub trait Storage {
    async fn read_all(&self) -> Result<Vec<DbOperation>, TaganrogError>;
    async fn write(&mut self, operation: DbOperation) -> Result<(), TaganrogError>;
}

pub struct FileStorage {
    db_path: PathBuf,
}

impl FileStorage {
    pub fn new(work_dir: PathBuf) -> Result<Self, ConfigError> {
        let db_path = work_dir.join("taganrog.db.json");
        if !db_path.exists() {
            std::fs::write(&db_path, "")?;
        }
        if db_path.exists() && !db_path.is_file() {
            return Err(ConfigError::Validation("db_path is not a file".to_string()));
        }
        Ok(Self { db_path })
    }
}

impl Storage for FileStorage {
    async fn read_all(&self) -> Result<Vec<DbOperation>, TaganrogError> {
        let file_str = tokio::fs::read_to_string(&self.db_path).await
            .map_err(TaganrogError::DbIOError)?;
        let operations = file_str.split('\n')
            .filter(|x| !x.is_empty())
            .map(|x| serde_json::from_str(x).map_err(TaganrogError::DbSerializationError))
            .collect::<Result<Vec<DbOperation>, TaganrogError>>()?;
        Ok(operations)
    }

    async fn write(&mut self, operation: DbOperation) -> Result<(), TaganrogError> {
        let serialized_operation = serde_json::to_string(&operation)
            .map_err(TaganrogError::DbSerializationError)?;
        let line = format!("{}\n", serialized_operation);
        let mut file = tokio::fs::OpenOptions::new().append(true).open(&self.db_path).await
            .map_err(TaganrogError::DbIOError)?;
        tokio::io::AsyncWriteExt::write_all(&mut file, line.as_bytes()).await
            .map_err(TaganrogError::DbIOError)?;
        Ok(())
    }
}
