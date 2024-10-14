use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaganrogError {
    #[error("Failed to read/write DB file: {0}")]
    DbIOError(std::io::Error),
    #[error("Failed to serialize/deserialize DB operation: {0}")]
    DbSerializationError(serde_json::Error),
    #[error("File not found")]
    FileNotFound,
    #[error("File read error: {0}")]
    FileReadError(std::io::Error),
    #[error("File metadata error: {0}")]
    FileMetadataError(std::io::Error),
}
