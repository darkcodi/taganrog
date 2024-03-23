use std::path::{Path, PathBuf};
use log::{error, info};
use crate::client::TaganrogClient;
use crate::config;
use crate::entities::{InsertResult, Media, MediaPage, TagsAutocomplete};
use crate::error::TaganrogError;
use crate::storage::Storage;
use crate::utils::normalize_query;

pub async fn tag_media<T: Storage>(client: &mut TaganrogClient<T>, filepath: &str, tag: &String) -> Result<bool, TaganrogError> {
    let canonical_filepath = canonicalize(filepath);
    let mut media = client.create_media_from_file(&canonical_filepath).await?;
    if client.get_media_by_id(&media.id).is_none() {
        media = client.add_media(media).await?.safe_unwrap();
    }
    let was_added = client.add_tag_to_media(&media.id, tag).await?;
    Ok(was_added)
}

pub async fn untag_media<T: Storage>(client: &mut TaganrogClient<T>, filepath: &str, tag: &String) -> Result<bool, TaganrogError> {
    let canonical_filepath = canonicalize(filepath);
    let media = client.create_media_from_file(&canonical_filepath).await?;
    let was_removed = client.remove_tag_from_media(&media.id, tag).await?;
    Ok(was_removed)
}

pub async fn list_tags<T: Storage>(client: &TaganrogClient<T>, tag_name: String, max_items: usize) -> Vec<TagsAutocomplete> {
    let normalized_query = normalize_query(&tag_name);
    if normalized_query.is_empty() {
        return client.get_all_tags();
    }
    client.autocomplete_tags(&normalized_query, max_items)
}

pub async fn search_media<T: Storage>(client: &TaganrogClient<T>, tags: Vec<String>, page_size: usize, page_index: usize) -> MediaPage {
    let query = tags.join(" ");
    let normalized_query = normalize_query(&query);
    if normalized_query.is_empty() {
        return client.get_all_media(page_size, page_index);
    }
    client.search_media(&normalized_query, page_size, page_index)
}

pub fn get_config_value(config_path: &Path, key: &str) {
    let file_config_result = config::read_file_config(config_path);
    if file_config_result.is_err() {
        error!("Failed to read config: {}", file_config_result.err().unwrap());
        std::process::exit(1);
    }
    let file_config = file_config_result.unwrap();
    match key {
        "work-dir" => {
            info!("Workdir: {:?}", file_config.workdir);
            std::process::exit(0);
        },
        "upload-dir" => {
            info!("Upload dir: {:?}", file_config.upload_dir);
            std::process::exit(0);
        },
        _ => {
            error!("Invalid key: {}", key);
            std::process::exit(1);
        }
    }
}

pub fn set_config_value(config_path: &Path, key: &str, value: &str) {
    let file_config_result = config::read_file_config(config_path);
    if file_config_result.is_err() {
        error!("Failed to read config: {}", file_config_result.err().unwrap());
        std::process::exit(1);
    }
    let mut file_config = file_config_result.unwrap();
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
            file_config.workdir = Some(path_str);
            let write_result = config::write_file_config(config_path, &file_config);
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
            file_config.upload_dir = Some(path_str);
            let write_result = config::write_file_config(config_path, &file_config);
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
