use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaganrogError {
    #[error("Failed to read/write DB file: {0}")]
    DbIOError(std::io::Error),
    #[error("Failed to serialize/deserialize DB operation: {0}")]
    DbSerializationError(serde_json::Error),
    #[error("Path is not within workdir")]
    PathNotWithinWorkdir,
    #[error("File not found")]
    FileNotFound,
    #[error("Path is a directory")]
    PathIsDirectory,
    #[error("Absolutize error")]
    AbsolutizeError,
    #[error("Relative path error")]
    RelativePathError,
    #[error("File read error: {0}")]
    FileReadError(std::io::Error),
    #[error("File metadata error: {0}")]
    FileMetadataError(std::io::Error),
    #[error("Filename is invalid")]
    InvalidFilename(String),
}
