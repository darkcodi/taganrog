use std::path::PathBuf;
use log::{error, info};
use crate::client::{TaganrogClient, TaganrogError};
use crate::config;
use crate::config::AppConfig;
use crate::entities::{InsertResult, Media};

pub async fn add_media(client: &mut TaganrogClient, filepath: &str) -> Result<InsertResult<Media>, TaganrogError> {
    let canonical_filepath = canonicalize(filepath);
    let media = client.create_media_from_file(&canonical_filepath).await?;
    let insert_result = client.add_media(media).await?;
    Ok(insert_result)
}

pub async fn remove_media(client: &mut TaganrogClient, filepath: &str) -> Result<Option<Media>, TaganrogError> {
    let canonical_filepath = canonicalize(filepath);
    let media = client.create_media_from_file(&canonical_filepath).await?;
    let media_id = media.id.clone();
    let maybe_media = client.delete_media(&media_id).await?;
    Ok(maybe_media)
}

pub fn get_config_value(config: AppConfig, key: &str) {
    match key {
        "work-dir" => {
            info!("Workdir: {:?}", config.file_config.workdir);
            std::process::exit(0);
        },
        "upload-dir" => {
            info!("Upload dir: {:?}", config.file_config.upload_dir);
            std::process::exit(0);
        },
        _ => {
            error!("Invalid key: {}", key);
            std::process::exit(1);
        }
    }
}

pub fn set_config_value(mut config: AppConfig, key: &str, value: &str) {
    match key {
        "work-dir" => {
            let path_result = PathBuf::try_from(value);
            if path_result.is_err() {
                error!("Invalid path: {}", value);
                std::process::exit(1);
            }
            let path = path_result.unwrap();
            if !path.exists() {
                error!("Path does not exist: {:?}", path);
                std::process::exit(1);
            }
            let path_str = path.display().to_string();
            config.file_config.workdir = Some(path_str);
            let write_result = config::write_file_config(&config.config_path, &config.file_config);
            if write_result.is_err() {
                error!("Failed to write config: {}", write_result.err().unwrap());
                std::process::exit(1);
            }
            info!("Workdir set to: {:?}", value);
            std::process::exit(0);
        },
        "upload-dir" => {
            let path_result = PathBuf::try_from(value);
            if path_result.is_err() {
                error!("Invalid path: {}", value);
                std::process::exit(1);
            }
            let path = path_result.unwrap();
            if !path.exists() {
                error!("Path does not exist: {:?}", path);
                std::process::exit(1);
            }
            let path_str = path.display().to_string();
            config.file_config.upload_dir = Some(path_str);
            let write_result = config::write_file_config(&config.config_path, &config.file_config);
            if write_result.is_err() {
                error!("Failed to write config: {}", write_result.err().unwrap());
                std::process::exit(1);
            }
            info!("Upload dir set to: {:?}", value);
            std::process::exit(0);
        },
        _ => {
            error!("Invalid key: {}", key);
            std::process::exit(1);
        }
    }
}

fn canonicalize(filepath: &str) -> PathBuf {
    let canonical_filepath_result = std::fs::canonicalize(filepath);
    if canonical_filepath_result.is_err() {
        let err = canonical_filepath_result.err().unwrap();
        if err.kind() == std::io::ErrorKind::NotFound {
            error!("File not found: {}", filepath);
            std::process::exit(1);
        } else {
            error!("IO Error: {}", err);
            std::process::exit(1);
        }
    }

    let canonical_filepath = canonical_filepath_result.unwrap();
    info!("File: {:?}", canonical_filepath);
    canonical_filepath
}
