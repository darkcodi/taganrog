use std::path::{PathBuf};
use serde::{Deserialize, Serialize};
use crate::entities::{Media, MediaId, Tag};
use crate::error::TaganrogError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbOperation {
    CreateMedia { media: Media },
    UpdateMedia { media: Media },
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
    pub fn new(db_path: PathBuf) -> anyhow::Result<Self> {
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

#[derive(Debug, Default)]
pub struct InMemoryStorage {
    operations: Vec<DbOperation>,
}

impl Storage for InMemoryStorage {
    async fn read_all(&self) -> Result<Vec<DbOperation>, TaganrogError> {
        Ok(self.operations.clone())
    }

    async fn write(&mut self, operation: DbOperation) -> Result<(), TaganrogError> {
        self.operations.push(operation);
        Ok(())
    }
}
