use std::collections::HashSet;
use std::path::{Path, PathBuf};
use dashmap::DashMap;
use itertools::Itertools;
use rand::seq::SliceRandom;
use tokio::time::Instant;
use crate::config::AppConfig;
use crate::entities::*;
use crate::error::TaganrogError;
use crate::storage::{DbOperation, Storage};
use crate::utils::hash_utils::MurMurHasher;

pub struct TaganrogClient<T: Storage> {
    cfg: AppConfig,

    // persistent storage
    storage: T,

    // in-memory storage
    media_map: DashMap<MediaId, Media>,
    tags_map: DashMap<Tag, HashSet<MediaId>>,
}

impl<T: Storage> TaganrogClient<T> {
    pub fn new(cfg: AppConfig, storage: T) -> Self {
        Self {
            cfg,
            storage,
            media_map: DashMap::new(),
            tags_map: DashMap::new(),
        }
    }

    pub async fn init(&mut self) -> Result<(), TaganrogError> {
        let operations = self.storage.read_all().await?;
        for operation in operations {
            match operation {
                DbOperation::CreateMedia { media } => { self.create_media_in_memory(media); }
                DbOperation::UpdateMedia { media } => { self.update_media_in_memory(media); }
                DbOperation::DeleteMedia { media_id } => { self.delete_media_in_memory(&media_id); }
                DbOperation::AddTag { media_id, tag } => { self.add_tag_to_media_in_memory(&media_id, &tag); }
                DbOperation::RemoveTag { media_id, tag } => { self.remove_tag_from_media_in_memory(&media_id, &tag); }
            }
        }
        Ok(())
    }

    pub fn get_media_count(&self) -> usize {
        self.media_map.len()
    }

    pub fn get_query_count(&self, tags: &[Tag]) -> usize {
        let intersection = self.get_media_intersection(tags);
        intersection.len()
    }

    fn get_media_intersection(&self, tags: &[Tag]) -> HashSet<MediaId> {
        if tags.is_empty() { return HashSet::new(); }

        // tag_map contains media_ids for each tag
        // we want to find the intersection of all media_ids for these tags
        let mut result = self.tags_map.get(&tags[0]).map(|x| x.value().clone()).unwrap_or_default();
        for tag in tags.iter().skip(1) {
            let media_ids = self.tags_map.get(tag).map(|x| x.value().clone()).unwrap_or_default();
            result.retain(|x| media_ids.contains(x));
        }
        result
    }

    pub fn get_media_by_id(&self, media_id: &MediaId) -> Option<Media> {
        let maybe_media = self.media_map.get(media_id).map(|x| x.value().clone());
        maybe_media
    }

    pub fn get_random_media(&self) -> Option<Media> {
        let media_vec = self.media_map.iter().map(|x| x.value().clone()).collect::<Vec<Media>>();
        let maybe_media = media_vec.choose(&mut rand::thread_rng()).cloned();
        maybe_media
    }

    pub fn get_all_media(&self, page_size: usize, page_index: usize) -> MediaPage {
        let start = Instant::now();
        let media_vec = self.media_map.iter()
            .map(|x| x.value().clone())
            .skip(page_index * page_size).take(page_size)
            .collect();
        let total_count = self.media_map.len();
        let total_pages = (total_count as f64 / page_size as f64).ceil() as usize;
        let elapsed = start.elapsed();
        MediaPage {
            media_vec,
            page_index,
            page_size,
            total_count,
            total_pages,
            elapsed: elapsed.as_millis() as u64,
        }
    }

    pub fn get_media_without_thumbnail(&self, page_size: usize, page_index: usize) -> MediaPage {
        let start = Instant::now();
        let media = self.media_map.iter()
            .filter(|x| !Path::new(&self.cfg.thumbnails_dir).join(format!("{}.png", &x.value().id)).exists())
            .map(|x| x.key().clone()).collect::<Vec<MediaId>>();
        let media_vec = media.iter()
            .skip(page_index * page_size).take(page_size)
            .map(|x| self.get_media_by_id(x).unwrap()).collect();
        let total_count = media.len();
        let total_pages = (total_count as f64 / page_size as f64).ceil() as usize;
        let elapsed = start.elapsed();
        MediaPage {
            media_vec,
            page_index,
            page_size,
            total_count,
            total_pages,
            elapsed: elapsed.as_millis() as u64,
        }
    }

    pub fn get_untagged_media(&self, page_size: usize, page_index: usize) -> MediaPage {
        let start = Instant::now();
        let media = self.media_map.iter()
            .filter(|x| x.value().tags.is_empty())
            .map(|x| x.key().clone()).collect::<Vec<MediaId>>();
        let media_vec = media.iter()
            .skip(page_index * page_size).take(page_size)
            .map(|x| self.get_media_by_id(x).unwrap()).collect();
        let total_count = self.media_map.iter().filter(|x| x.value().tags.is_empty()).count();
        let total_pages = (total_count as f64 / page_size as f64).ceil() as usize;
        let elapsed = start.elapsed();
        MediaPage {
            media_vec,
            page_index,
            page_size,
            total_count,
            total_pages,
            elapsed: elapsed.as_millis() as u64,
        }
    }

    pub fn search_media(&self, query: &str, page_size: usize, page_index: usize) -> MediaPage {
        let start = Instant::now();
        if query.is_empty() {
            return MediaPage::default();
        }
        let query_arr = query.split(' ').collect::<Vec<&str>>();
        if query_arr.is_empty() {
            return MediaPage::default();
        }
        let exact_match_tags = query_arr.iter()
            .map(|x| x.to_string())
            .collect::<Vec<Tag>>();
        let has_unknown_tag = exact_match_tags.iter().any(|x| !self.tags_map.contains_key(x));
        if has_unknown_tag {
            return MediaPage::default();
        }
        let intersection = self.get_media_intersection(&exact_match_tags);
        let media_vec = intersection.iter()
            .sorted_by_key(|x| self.media_map.get(*x).unwrap().created_at).rev()
            .skip(page_index * page_size).take(page_size)
            .map(|x| self.get_media_by_id(x).unwrap())
            .collect();
        let elapsed = start.elapsed();
        let total_count = intersection.len();
        let total_pages = (total_count as f64 / page_size as f64).ceil() as usize;
        MediaPage {
            media_vec,
            page_index,
            page_size,
            total_count,
            total_pages,
            elapsed: elapsed.as_millis() as u64,
        }
    }

    pub fn get_all_tags(&self) -> Vec<TagsAutocomplete> {
        self.tags_map.iter()
            .map(|x| {
                let media_count = x.value().len();
                TagsAutocomplete {
                    head: vec![],
                    last: x.key().clone(),
                    media_count,
                }
            })
            .filter(|x| x.media_count > 0)
            .sorted_by_key(|x| x.last.clone()).rev()
            .sorted_by_key(|x| x.media_count).rev()
            .collect()
    }

    pub fn autocomplete_tags(&self, query: &str, max_items: usize) -> Vec<TagsAutocomplete> {
        if query.is_empty() {
            return vec![];
        }
        let query_arr = query.split(' ').collect::<Vec<&str>>();
        if query_arr.is_empty() {
            return vec![];
        }
        let exact_match_tags = query_arr.iter()
            .take(query_arr.len() - 1)
            .map(|x| x.to_string())
            .collect::<Vec<Tag>>();
        let has_unknown_tag = exact_match_tags.iter().any(|x| !self.tags_map.contains_key(x));
        if has_unknown_tag {
            return vec![];
        }
        let last_tag = query_arr.last().unwrap().to_string();
        let possible_last_tags = self.tags_map.iter()
            .filter(|x| x.key().starts_with(&last_tag))
            .filter(|x| !exact_match_tags.contains(x.key()))
            .map(|x| x.key().clone())
            .collect::<Vec<Tag>>();
        let matching_media_ids = if exact_match_tags.is_empty() {
            self.media_map.iter().map(|x| x.key().clone()).collect()
        } else {
            self.get_media_intersection(&exact_match_tags)
        };
        let autocomplete = possible_last_tags.iter()
            .map(|x| {
                let media_ids = self.tags_map.get(x).map(|x| x.value().clone()).unwrap_or_default();
                let count = media_ids.intersection(&matching_media_ids).count();
                TagsAutocomplete {
                    head: exact_match_tags.clone(),
                    last: x.clone(),
                    media_count: count,
                }
            })
            .sorted_by_key(|x| x.media_count).rev()
            .take(max_items)
            .filter(|x| x.media_count > 0)
            .collect();
        autocomplete
    }

    fn create_media_in_memory(&mut self, media: Media) -> InsertResult<Media> {
        let id = media.id.clone();
        if self.media_map.contains_key(&id) {
            return InsertResult::Existing(media);
        }
        self.media_map.insert(id, media.clone());
        InsertResult::New(media)
    }

    fn update_media_in_memory(&mut self, media: Media) -> InsertResult<Media> {
        let id = media.id.clone();
        if self.media_map.contains_key(&id) {
            self.media_map.remove(&id);
            self.media_map.insert(id, media.clone());
            InsertResult::Existing(media)
        } else {
            self.media_map.insert(id, media.clone());
            InsertResult::New(media)
        }
    }

    fn delete_media_in_memory(&mut self, media_id: &MediaId) -> Option<Media> {
        let maybe_media = self.media_map.remove(media_id);
        if maybe_media.is_none() {
            return None;
        }
        let media = maybe_media.unwrap().1;
        for tag in media.tags.iter() {
            let mut entry = self.tags_map.entry(tag.clone()).or_default();
            entry.value_mut().remove(media_id);
        }
        Some(media)
    }

    fn add_tag_to_media_in_memory(&mut self, media_id: &MediaId, tag: &Tag) -> bool {
        let maybe_media = self.media_map.get_mut(media_id);
        if let Some(mut kvp) = maybe_media {
            let media = kvp.value_mut();
            if !media.tags.contains(tag) {
                media.tags.push(tag.clone());
                self.tags_map.entry(tag.clone()).or_default().value_mut().insert(media_id.clone());
                return true;
            }
        }
        false
    }

    fn remove_tag_from_media_in_memory(&mut self, media_id: &MediaId, tag: &Tag) -> bool {
        let maybe_media = self.media_map.get_mut(media_id);
        if let Some(mut kvp) = maybe_media {
            let media = kvp.value_mut();
            if media.tags.contains(tag) {
                media.tags.retain(|x| x != tag);
                let mut entry = self.tags_map.entry(tag.clone()).or_default();
                entry.value_mut().remove(media_id);
                return true;
            }
        }
        false
    }

    pub async fn create_media_from_file(&self, abs_path: &PathBuf) -> Result<Media, TaganrogError> {
        if !abs_path.exists() || abs_path.is_dir() {
            return Err(TaganrogError::FileNotFound);
        }

        let file_bytes = std::fs::read(&abs_path)
            .map_err(TaganrogError::FileReadError)?;
        let hash = MurMurHasher::hash_bytes(&file_bytes);
        let metadata = std::fs::metadata(&abs_path)
            .map_err(TaganrogError::FileMetadataError)?;
        let content_type = infer::get(&file_bytes).map(|x| x.mime_type()).unwrap_or("application/octet-stream").to_string();
        let size = metadata.len() as i64;
        let location = abs_path.to_string_lossy().to_string();
        let created_at = chrono::Utc::now();
        let filename = abs_path.file_name().unwrap().to_string_lossy().to_string();

        let media = Media {
            id: hash,
            filename,
            content_type,
            created_at,
            size,
            location,
            tags: vec![],
        };

        Ok(media)
    }

    pub async fn add_media(&mut self, media: Media) -> Result<InsertResult<Media>, TaganrogError> {
        let hash = media.id.clone();
        if self.media_map.contains_key(&hash) {
            return Ok(InsertResult::Existing(self.media_map.get(&hash).map(|x| x.value().clone()).unwrap()));
        }
        let result = self.create_media_in_memory(media.clone());
        if let InsertResult::New(media) = &result {
            self.storage.write(DbOperation::CreateMedia { media: media.clone() }).await?;
        }
        Ok(result)
    }

    pub async fn update_media(&mut self, media: Media) -> Result<InsertResult<Media>, TaganrogError> {
        let result = self.update_media_in_memory(media);
        self.storage.write(DbOperation::UpdateMedia { media: result.clone().safe_unwrap() }).await?;
        Ok(result)
    }

    pub async fn delete_media(&mut self, media_id: &MediaId) -> Result<Option<Media>, TaganrogError> {
        let maybe_media = self.delete_media_in_memory(media_id);
        if let Some(media) = &maybe_media {
            self.storage.write(DbOperation::DeleteMedia { media_id: media.id.clone() }).await?;
        }
        Ok(maybe_media)
    }

    pub async fn add_tag_to_media(&mut self, media_id: &MediaId, tag: &Tag) -> Result<bool, TaganrogError> {
        let was_added = self.add_tag_to_media_in_memory(media_id, tag);
        if was_added {
            self.storage.write(DbOperation::AddTag { media_id: media_id.clone(), tag: tag.clone() }).await?;
        }
        Ok(was_added)
    }

    pub async fn remove_tag_from_media(&mut self, media_id: &MediaId, tag: &Tag) -> Result<bool, TaganrogError> {
        let was_removed = self.remove_tag_from_media_in_memory(media_id, tag);
        if was_removed {
            self.storage.write(DbOperation::RemoveTag { media_id: media_id.clone(), tag: tag.clone() }).await?;
        }
        Ok(was_removed)
    }

    pub fn get_media_path(&self, media_id: &MediaId) -> Option<PathBuf> {
        let media = self.get_media_by_id(media_id)?;
        let media_path = PathBuf::from(&media.location);
        Some(media_path)
    }

    pub fn get_thumbnail_path(&self, media_id: &MediaId) -> PathBuf {
        self.cfg.thumbnails_dir.join(format!("{}.png", media_id))
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use super::*;
    use tempfile::tempdir;
    use crate::storage::InMemoryStorage;

    async fn create_test_client() -> TaganrogClient<InMemoryStorage> {
        let dir = tempdir().unwrap();
        let cfg = AppConfig::new(dir.path().to_path_buf());
        let storage = InMemoryStorage::default();
        let mut client = TaganrogClient::new(cfg, storage);
        client.init().await.unwrap();
        client
    }

    fn create_random_media() -> Media {
        let id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric)
            .take(32).map(char::from).collect();
        let filename = "test.txt".to_string();
        let content_type = "text/plain".to_string();
        let created_at = chrono::Utc::now();
        let size = 0;
        let location = "test.txt".to_string();
        let tags = vec![];
        Media { id, filename, content_type, created_at, size, location, tags }
    }

    #[tokio::test]
    async fn test_get_media_count() {
        let mut client = create_test_client().await;
        assert_eq!(client.get_media_count(), 0);
        let media = create_random_media();
        client.create_media_in_memory(media.clone());
        assert_eq!(client.get_media_count(), 1);
        client.delete_media_in_memory(&media.id);
        assert_eq!(client.get_media_count(), 0);
    }

    #[tokio::test]
    async fn test_get_query_count() {
        let mut client = create_test_client().await;
        let media = create_random_media();
        client.create_media_in_memory(media.clone());
        client.add_tag_to_media_in_memory(&media.id, &"tag1".to_string());
        client.add_tag_to_media_in_memory(&media.id, &"tag2".to_string());
        assert_eq!(client.get_query_count(&["tag1".to_string()]), 1);
        assert_eq!(client.get_query_count(&["tag1".to_string(), "tag2".to_string()]), 1);
        assert_eq!(client.get_query_count(&["tag1".to_string(), "tag2".to_string(), "tag3".to_string()]), 0);
        assert_eq!(client.get_query_count(&["tag3".to_string()]), 0);
    }

    #[tokio::test]
    async fn test_get_media_intersection() {
        let mut client = create_test_client().await;
        let media1 = create_random_media();
        let media2 = create_random_media();
        client.create_media_in_memory(media1.clone());
        client.create_media_in_memory(media2.clone());
        client.add_tag_to_media_in_memory(&media1.id, &"tag1".to_string());
        client.add_tag_to_media_in_memory(&media2.id, &"tag2".to_string());
        client.add_tag_to_media_in_memory(&media1.id, &"tag2".to_string());
        let intersection = client.get_media_intersection(&["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(intersection.len(), 1);
        assert!(intersection.contains(&media1.id));
    }

    #[tokio::test]
    async fn test_get_media_by_id() {
        let mut client = create_test_client().await;
        let media = create_random_media();
        client.create_media_in_memory(media.clone());
        let maybe_media = client.get_media_by_id(&media.id);
        assert_eq!(maybe_media, Some(media));
    }

    #[tokio::test]
    async fn test_get_random_media() {
        let mut client = create_test_client().await;
        let media = create_random_media();
        client.create_media_in_memory(media.clone());
        let maybe_media = client.get_random_media();
        assert_eq!(maybe_media, Some(media));
    }

    #[tokio::test]
    async fn test_get_all_media() {
        let mut client = create_test_client().await;
        let media1 = create_random_media();
        let media2 = create_random_media();
        client.create_media_in_memory(media1.clone());
        client.create_media_in_memory(media2.clone());
        let page = client.get_all_media(10, 0);
        assert_eq!(page.media_vec.len(), 2);
        assert_eq!(page.total_count, 2);
        assert_eq!(page.total_pages, 1);
    }

    #[tokio::test]
    async fn test_get_media_without_thumbnail() {
        let mut client = create_test_client().await;
        let media1 = create_random_media();
        let media2 = create_random_media();
        client.create_media_in_memory(media1.clone());
        client.create_media_in_memory(media2.clone());
        let page = client.get_media_without_thumbnail(10, 0);
        assert_eq!(page.media_vec.len(), 2);
        assert_eq!(page.total_count, 2);
        assert_eq!(page.total_pages, 1);
    }

    #[tokio::test]
    async fn test_get_untagged_media() {
        let mut client = create_test_client().await;
        let media1 = create_random_media();
        let media2 = create_random_media();
        client.create_media_in_memory(media1.clone());
        client.create_media_in_memory(media2.clone());
        let page = client.get_untagged_media(10, 0);
        assert_eq!(page.media_vec.len(), 2);
        assert_eq!(page.total_count, 2);
        assert_eq!(page.total_pages, 1);

        client.add_tag_to_media_in_memory(&media1.id, &"tag1".to_string());
        let page = client.get_untagged_media(10, 0);
        assert_eq!(page.media_vec.len(), 1);
        assert_eq!(page.total_count, 1);
        assert_eq!(page.total_pages, 1);
    }

    #[tokio::test]
    async fn test_search_media() {
        let mut client = create_test_client().await;
        let media1 = create_random_media();
        let media2 = create_random_media();
        client.create_media_in_memory(media1.clone());
        client.create_media_in_memory(media2.clone());
        client.add_tag_to_media_in_memory(&media1.id, &"tag1".to_string());
        client.add_tag_to_media_in_memory(&media2.id, &"tag2".to_string());
        let page = client.search_media("tag1", 10, 0);
        assert_eq!(page.media_vec.len(), 1);
        assert_eq!(page.total_count, 1);
        assert_eq!(page.total_pages, 1);
    }

    #[tokio::test]
    async fn test_get_all_tags() {
        let mut client = create_test_client().await;
        let media1 = create_random_media();
        let media2 = create_random_media();
        client.create_media_in_memory(media1.clone());
        client.create_media_in_memory(media2.clone());
        client.add_tag_to_media_in_memory(&media1.id, &"tag1".to_string());
        client.add_tag_to_media_in_memory(&media2.id, &"tag2".to_string());
        let tags = client.get_all_tags();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].last, "tag1");
        assert_eq!(tags[0].media_count, 1);
        assert_eq!(tags[1].last, "tag2");
        assert_eq!(tags[1].media_count, 1);
    }

    #[tokio::test]
    async fn test_autocomplete_tags() {
        let mut client = create_test_client().await;
        let media1 = create_random_media();
        let media2 = create_random_media();
        client.create_media_in_memory(media1.clone());
        client.create_media_in_memory(media2.clone());
        client.add_tag_to_media_in_memory(&media1.id, &"tag1".to_string());
        client.add_tag_to_media_in_memory(&media2.id, &"tag2".to_string());
        let mut tags = client.autocomplete_tags("tag", 10);
        tags.sort_by(|a, b| a.last.cmp(&b.last));
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].last, "tag1");
        assert_eq!(tags[0].media_count, 1);
        assert_eq!(tags[1].last, "tag2");
        assert_eq!(tags[1].media_count, 1);
    }
}
