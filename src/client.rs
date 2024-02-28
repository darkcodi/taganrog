use std::collections::HashSet;
use std::path::{Path, PathBuf};
use dashmap::DashMap;
use itertools::Itertools;
use path_absolutize::Absolutize;
use serde::{Deserialize, Serialize};
use tracing::info;
use rand::seq::SliceRandom;
use relative_path::PathExt;
use crate::entities::*;
use crate::utils::hash_utils::MurMurHasher;

#[derive(Debug, Serialize, Deserialize)]
enum DbOperation {
    CreateMedia { media: Media },
    DeleteMedia { media_id: MediaId },
    AddTag { media_id: MediaId, tag: Tag },
    RemoveTag { media_id: MediaId, tag: Tag },
}

#[derive(Debug)]
pub struct TaganrogConfig {
    pub workdir: PathBuf,
    pub upload_dir: PathBuf,
    pub db_path: PathBuf,
    pub thumbnails_dir: PathBuf,
}

impl TaganrogConfig {
    pub fn new(workdir: &str, upload_dir: &str) -> anyhow::Result<Self> {
        let workdir = Self::get_or_create_workdir(workdir)?;
        let upload_dir = Self::get_or_create_upload_dir(&workdir, upload_dir)?;
        let db_path = Self::get_or_create_db_path(&workdir)?;
        let thumbnails_dir = Self::get_or_create_thumbnails_dir(&workdir)?;
        Ok(Self { workdir, upload_dir, db_path, thumbnails_dir })
    }

    fn get_or_create_workdir(workdir: &str) -> anyhow::Result<PathBuf> {
        let workdir = Path::new(workdir).absolutize_from(std::env::current_dir()?)?.canonicalize()?;
        if !workdir.exists() {
            std::fs::create_dir_all(&workdir)?;
        }
        if !workdir.is_dir() {
            anyhow::bail!("workdir is not a directory");
        }
        info!("workdir: {}", workdir.display());
        Ok(workdir)
    }

    fn get_or_create_upload_dir(workdir: &PathBuf, upload_dir: &str) -> anyhow::Result<PathBuf> {
        let upload_dir = Path::new(upload_dir).absolutize_from(std::env::current_dir()?)?.canonicalize()?;
        if !upload_dir.starts_with(workdir) {
            anyhow::bail!("upload_dir is not a subdirectory of workdir");
        }
        if !upload_dir.exists() {
            std::fs::create_dir_all(&upload_dir)?;
        }
        if upload_dir.exists() && !upload_dir.is_dir() {
            anyhow::bail!("upload_dir is not a directory");
        }
        info!("upload_dir: {}", upload_dir.display());
        Ok(upload_dir)
    }

    fn get_or_create_db_path(workdir: &Path) -> anyhow::Result<PathBuf> {
        let db_path = workdir.join("taganrog.db.json");
        if !db_path.exists() {
            std::fs::write(&db_path, "")?;
        }
        if db_path.exists() && !db_path.is_file() {
            anyhow::bail!("db_path is not a file");
        }
        info!("db_path: {}", workdir.display());
        Ok(db_path)
    }

    fn get_or_create_thumbnails_dir(workdir: &Path) -> anyhow::Result<PathBuf> {
        let thumbnails_dir = workdir.join("taganrog-thumbnails");
        if !thumbnails_dir.exists() {
            std::fs::create_dir_all(&thumbnails_dir)?;
        }
        if thumbnails_dir.exists() && !thumbnails_dir.is_dir() {
            anyhow::bail!("thumbnails_dir is not a directory");
        }
        info!("thumbnails_dir: {}", thumbnails_dir.display());
        Ok(thumbnails_dir)
    }
}

pub struct TaganrogClient {
    cfg: TaganrogConfig,
    media_map: DashMap<MediaId, Media>,
    tags_map: DashMap<Tag, HashSet<MediaId>>,
}

impl TaganrogClient {
    pub fn new(cfg: TaganrogConfig) -> Self {
        Self {
            cfg,
            media_map: DashMap::new(),
            tags_map: DashMap::new(),
        }
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
        info!("Starting DB import from WAL...");
        let file_str = tokio::fs::read_to_string(&self.cfg.db_path).await?;
        for line in file_str.split('\n') {
            if line.is_empty() {
                continue;
            }
            let operation: DbOperation = serde_json::from_str(line)?;
            match operation {
                DbOperation::CreateMedia { media } => { self.create_media_no_wal(media); }
                DbOperation::DeleteMedia { media_id } => { self.delete_media_no_wal(&media_id); }
                DbOperation::AddTag { media_id, tag } => { self.add_tag_to_media_no_wal(&media_id, &tag); }
                DbOperation::RemoveTag { media_id, tag } => { self.remove_tag_from_media_no_wal(&media_id, &tag); }
            }
        }
        info!("DB Imported!");
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
        // start with a tag that has the least media_ids
        let mut result = self.tags_map.get(&tags[0]).map(|x| x.value().clone()).unwrap_or_default();
        for tag in tags.iter().skip(1) {
            let media_ids = self.tags_map.get(tag).map(|x| x.value().clone()).unwrap_or_default();
            result.retain(|x| media_ids.contains(x));
        }
        result
    }

    async fn write_wal(&mut self, operation: DbOperation) -> anyhow::Result<()> {
        info!("Writing to WAL: {:?}", operation);
        let serialized_operation = serde_json::to_string(&operation)?;
        let line = format!("{}\n", serialized_operation);
        let mut file = tokio::fs::OpenOptions::new().append(true).open(&self.cfg.db_path).await?;
        tokio::io::AsyncWriteExt::write_all(&mut file, line.as_bytes()).await?;
        info!("WAL written!");
        Ok(())
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

    pub fn get_all_media(&self, page_size: usize, page_index: usize) -> Vec<Media> {
        let media_vec = self.media_map.iter()
            .skip(page_index * page_size).take(page_size)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    pub fn get_media_without_thumbnail(&self, page_size: usize, page_index: usize) -> Vec<Media> {
        let media_vec = self.media_map.iter()
            .filter(|x| !Path::new(&self.cfg.thumbnails_dir).join(format!("{}.png", &x.value().id)).exists())
            .skip(page_index * page_size).take(page_size)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    pub fn get_untagged_media(&self, page_size: usize, page_index: usize) -> Vec<Media> {
        let media_vec = self.media_map.iter()
            .filter(|x| x.value().tags.is_empty())
            .skip(page_index * page_size).take(page_size)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    pub fn search_media(&self, query: &str, page_size: usize, page_index: usize) -> Vec<Media> {
        if query.is_empty() {
            return vec![];
        }
        let query_arr = query.split(' ').collect::<Vec<&str>>();
        if query_arr.is_empty() {
            return vec![];
        }
        let exact_match_tags = query_arr.iter()
            .map(|x| x.to_string())
            .collect::<Vec<Tag>>();
        let has_unknown_tag = exact_match_tags.iter().any(|x| !self.tags_map.contains_key(x));
        if has_unknown_tag {
            return vec![];
        }
        let intersection = self.get_media_intersection(&exact_match_tags);
        let media_vec = intersection.iter()
            .skip(page_index * page_size).take(page_size)
            .map(|x| self.get_media_by_id(x).unwrap())
            .collect();
        media_vec
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

    fn create_media_no_wal(&mut self, media: Media) -> InsertResult<Media> {
        let id = media.id.clone();
        if self.media_map.contains_key(&id) {
            return InsertResult::Existing(media);
        }
        self.media_map.insert(id.clone(), media.clone());
        InsertResult::New(media)
    }

    fn delete_media_no_wal(&mut self, media_id: &MediaId) -> Option<Media> {
        self.media_map.remove(media_id).map(|x| x.1)
    }

    fn add_tag_to_media_no_wal(&mut self, media_id: &MediaId, tag: &Tag) -> bool {
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

    fn remove_tag_from_media_no_wal(&mut self, media_id: &MediaId, tag: &Tag) -> bool {
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

    pub async fn upload_media(&mut self, content: Vec<u8>, filename: String) -> anyhow::Result<InsertResult<Media>> {
        if filename.is_empty() {
            return Err(anyhow::anyhow!("Filename is empty"));
        }
        if filename.contains('/') || filename.contains('\\') {
            return Err(anyhow::anyhow!("Filename contains invalid characters"));
        }
        let hash = MurMurHasher::hash_bytes(&content);
        if self.media_map.contains_key(&hash) {
            return Ok(InsertResult::Existing(self.media_map.get(&hash).map(|x| x.value().clone()).unwrap()));
        }
        let mut unique_filename = filename.clone();
        let mut suffix = 0;
        while self.cfg.upload_dir.join(&unique_filename).exists() {
            suffix += 1;
            unique_filename = format!("dup{}-{}", suffix, filename);
        }
        let abs_path = self.cfg.upload_dir.join(&unique_filename);
        tokio::fs::write(&abs_path, &content).await?;
        let rel_path = abs_path.relative_to(&self.cfg.workdir)?;
        let content_type = infer::get(&content).map(|x| x.mime_type()).unwrap_or("application/octet-stream").to_string();
        let size = content.len() as i64;
        let location = rel_path.to_string();
        let created_at = chrono::Utc::now();

        let media = Media {
            id: hash,
            filename: unique_filename,
            content_type,
            created_at,
            size,
            location,
            was_uploaded: true,
            tags: vec![],
        };

        let result = self.create_media_no_wal(media.clone());
        if let InsertResult::New(media) = &result {
            self.write_wal(DbOperation::CreateMedia { media: media.clone() }).await?;
        }
        Ok(result)
    }

    pub async fn create_media_from_file(&mut self, abs_path: PathBuf) -> anyhow::Result<InsertResult<Media>> {
        let abs_path = abs_path.absolutize()?;
        let rel_path = abs_path.relative_to(&self.cfg.workdir)?;
        if rel_path.starts_with(".") {
            return Err(anyhow::anyhow!("Path is not within workdir"));
        }
        if !abs_path.exists() {
            return Err(anyhow::anyhow!("File does not exist"));
        }
        if abs_path.is_dir() {
            return Err(anyhow::anyhow!("Path is a directory"));
        }

        let file_bytes = std::fs::read(&abs_path)?;
        let hash = MurMurHasher::hash_bytes(&file_bytes);
        if self.media_map.contains_key(&hash) {
            return Ok(InsertResult::Existing(self.media_map.get(&hash).map(|x| x.value().clone()).unwrap()));
        }
        let metadata = std::fs::metadata(&abs_path)?;
        let content_type = infer::get(&file_bytes).map(|x| x.mime_type()).unwrap_or("application/octet-stream").to_string();
        let size = metadata.len() as i64;
        let location = rel_path.to_string();
        let created_at = chrono::Utc::now();
        let filename = abs_path.file_name().unwrap().to_string_lossy().to_string();

        let media = Media {
            id: hash,
            filename,
            content_type,
            created_at,
            size,
            location,
            was_uploaded: false,
            tags: vec![],
        };

        let result = self.create_media_no_wal(media.clone());
        if let InsertResult::New(media) = &result {
            self.write_wal(DbOperation::CreateMedia { media: media.clone() }).await?;
        }
        Ok(result)
    }

    pub async fn delete_media(&mut self, media_id: &MediaId) -> anyhow::Result<Option<Media>> {
        let maybe_media = self.delete_media_no_wal(media_id);
        if let Some(media) = &maybe_media {
            self.write_wal(DbOperation::DeleteMedia { media_id: media.id.clone() }).await?;
        }
        Ok(maybe_media)
    }

    pub async fn add_tag_to_media(&mut self, media_id: &MediaId, tag: &Tag) -> anyhow::Result<bool> {
        let result = self.add_tag_to_media_no_wal(media_id, tag);
        if result {
            self.write_wal(DbOperation::AddTag { media_id: media_id.clone(), tag: tag.clone() }).await?;
        }
        Ok(result)
    }

    pub async fn remove_tag_from_media(&mut self, media_id: &MediaId, tag: &Tag) -> anyhow::Result<bool> {
        let result = self.remove_tag_from_media_no_wal(media_id, tag);
        if result {
            self.write_wal(DbOperation::RemoveTag { media_id: media_id.clone(), tag: tag.clone() }).await?;
        }
        Ok(result)
    }

    pub fn get_media_path(&self, media_id: &MediaId) -> Option<PathBuf> {
        let media = self.get_media_by_id(media_id)?;
        let media_path = self.cfg.workdir.join(media.location);
        Some(media_path)
    }

    pub fn get_thumbnail_path(&self, media_id: &MediaId) -> PathBuf {
        self.cfg.thumbnails_dir.join(format!("{}.png", media_id))
    }
}
