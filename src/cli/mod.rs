use std::path::PathBuf;
use crate::client::TaganrogClient;
use crate::entities::{MediaPage, TagsAutocomplete};
use crate::error::TaganrogError;
use crate::storage::Storage;
use crate::utils::normalize_query;

pub async fn tag_media<T: Storage>(client: &mut TaganrogClient<T>, filepath: &str, tag: &String) -> Result<bool, TaganrogError> {
    let filepath: PathBuf = filepath.into();
    let mut media = client.create_media_from_file(&filepath).await?;
    if client.get_media_by_id(&media.id).is_none() {
        media = client.add_media(media).await?.safe_unwrap();
    }
    let was_added = client.add_tag_to_media(&media.id, tag).await?;
    Ok(was_added)
}

pub async fn untag_media<T: Storage>(client: &mut TaganrogClient<T>, filepath: &str, tag: &String) -> Result<bool, TaganrogError> {
    let filepath: PathBuf = filepath.into();
    let media = client.create_media_from_file(&filepath).await?;
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
