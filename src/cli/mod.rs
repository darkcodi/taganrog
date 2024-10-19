use std::path::PathBuf;
use log::{error, info};
use crate::client::TaganrogClient;
use crate::entities::{MediaPage, TagsAutocomplete};
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

fn canonicalize(filepath: &str) -> PathBuf {
    let canonical_filepath_result = std::fs::canonicalize(filepath);
    if canonical_filepath_result.is_err() {
        let err = canonical_filepath_result.err().unwrap();
        if err.kind() == std::io::ErrorKind::NotFound {
            error!("file not found: {}", filepath);
            std::process::exit(1);
        } else {
            error!("io error: {}", err);
            std::process::exit(1);
        }
    }

    let canonical_filepath = canonical_filepath_result.unwrap();
    info!("file: {:?}", canonical_filepath);
    canonical_filepath
}
