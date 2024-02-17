use std::path::PathBuf;
use dashmap::DashMap;
use indicium::simple::{AutocompleteType, EddieMetric, SearchIndex, SearchType, StrsimMetric};
use path_absolutize::Absolutize;
use serde::{Deserialize, Serialize};
use tracing::info;
use rand::seq::SliceRandom;
use relative_path::PathExt;
use crate::entities::{InsertResult, Media, TagsAutocomplete};
use crate::utils::hash_utils::MurMurHasher;

#[derive(Debug, Serialize, Deserialize)]
enum DbOperation {
    CreateMedia { media: Media },
    DeleteMedia { media_id: String },
    AddTagToMedia { media_id: String, tag: String },
    RemoveTagFromMedia { media_id: String, tag: String },
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
        let upload_dir = Self::get_or_create_upload_dir(&workdir, &upload_dir)?;
        let db_path = Self::get_or_create_db_path(&workdir)?;
        let thumbnails_dir = Self::get_or_create_thumbnails_dir(&workdir)?;
        Ok(Self { workdir, upload_dir, db_path, thumbnails_dir })
    }

    fn get_or_create_workdir(workdir: &str) -> anyhow::Result<PathBuf> {
        let workdir = std::path::Path::new(workdir).absolutize_from(std::env::current_dir()?)?.canonicalize()?;
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
        let upload_dir = std::path::Path::new(upload_dir).absolutize_from(std::env::current_dir()?)?.canonicalize()?;
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

    fn get_or_create_db_path(workdir: &PathBuf) -> anyhow::Result<PathBuf> {
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

    fn get_or_create_thumbnails_dir(workdir: &PathBuf) -> anyhow::Result<PathBuf> {
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
    map: DashMap<String, Media>,
    index: SearchIndex<String>,
}

impl TaganrogClient {
    pub fn new(cfg: TaganrogConfig) -> Self {
        Self {
            cfg,
            map: DashMap::new(),
            index: SearchIndex::new(
                SearchType::Live,
                AutocompleteType::Context,
                Some(StrsimMetric::Levenshtein),
                Some(EddieMetric::Levenshtein),
                3,
                0.3,
                Some(vec!['\t', '\n', '\r', ' ', '!', '"', '&', '(', ')', '*', '+', ',', '.', '/', ':', ';', '<', '=', '>', '?', '[', '\\', ']', '^', '`', '{', '|', '}', '~', ' ', '¡', '«', '»', '¿', '×', '÷', 'ˆ', '‘', '’', '“', '”', '„', '‹', '›', '—', ]),
                false,
                0,
                30,
                Some(30),
                None,
                10,
                20,
                40_960,
                None,
            ),
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
                DbOperation::AddTagToMedia { media_id, tag } => { self.add_tag_to_media_no_wal(&media_id, &tag); }
                DbOperation::RemoveTagFromMedia { media_id, tag } => { self.remove_tag_from_media_no_wal(&media_id, &tag); }
            }
        }
        info!("DB Imported!");
        Ok(())
    }

    pub fn get_media_count(&self) -> usize {
        self.map.len()
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

    pub fn get_media_by_id(&self, media_id: &String) -> Option<Media> {
        let maybe_media = self.map.get(media_id).map(|x| x.value().clone());
        maybe_media
    }

    pub fn get_random_media(&self) -> Option<Media> {
        let media_vec = self.map.iter().map(|x| x.value().clone()).collect::<Vec<Media>>();
        let maybe_media = media_vec.choose(&mut rand::thread_rng()).map(|x| x.clone());
        maybe_media
    }

    pub fn get_all_media(&self, page_size: usize, page_index: usize) -> Vec<Media> {
        let media_vec = self.map.iter()
            .skip(page_index * page_size).take(page_size)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    pub fn get_media_without_thumbnail(&self, page_size: usize, page_index: usize) -> Vec<Media> {
        let media_vec = self.map.iter()
            .filter(|x| !std::path::Path::new(&self.cfg.thumbnails_dir).join(format!("{}.png", &x.value().id)).exists())
            .skip(page_index * page_size).take(page_size)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    pub fn get_untagged_media(&self, page_size: usize, page_index: usize) -> Vec<Media> {
        let media_vec = self.map.iter()
            .filter(|x| x.value().tags.is_empty())
            .skip(page_index * page_size).take(page_size)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    pub fn search_media(&self, query: &String, page_size: usize, page_index: usize) -> Vec<Media> {
        let skip_results = page_size * page_index;
        let max_results = skip_results + page_size;
        let media_ids = self.index.search_with(&SearchType::Live, &max_results, &query);
        let media_vec = media_ids.into_iter()
            .take(max_results).skip(skip_results)
            .map(|x| self.map.get(x).map(|x| x.value().clone())).flatten().collect();
        media_vec
    }

    pub fn autocomplete_tags(&self, query: &String, max_items: usize) -> Vec<TagsAutocomplete> {
        let autocomplete = self.index.autocomplete_with(&AutocompleteType::Context, &max_items, query);
        let res = autocomplete.into_iter().map(|str| {
            let mut split = str.split(' ').map(|x| x.to_string()).collect::<Vec<String>>();
            let last = split.pop().unwrap();
            let head = split;
            TagsAutocomplete { head, last }
        }).collect();
        res
    }

    fn create_media_no_wal(&mut self, media: Media) -> InsertResult<Media> {
        let id = media.id.clone();
        if self.map.contains_key(&id) {
            return InsertResult::Existing(media);
        }
        self.map.insert(id.clone(), media.clone());
        self.index.insert(&id, &media);
        InsertResult::New(media)
    }

    fn delete_media_no_wal(&mut self, media_id: &String) -> Option<Media> {
        let maybe_media = self.map.remove(media_id);
        if let Some((id, media)) = maybe_media {
            self.index.remove(&id, &media);
            return Some(media);
        }
        None
    }

    fn add_tag_to_media_no_wal(&mut self, media_id: &String, tag: &String) -> bool {
        let maybe_media = self.map.get_mut(media_id);
        if let Some(mut kvp) = maybe_media {
            let media = kvp.value_mut();
            if !media.tags.contains(tag) {
                media.tags.push(tag.clone());
                self.index.insert(media_id, media);
                return true;
            }
        }
        false
    }

    fn remove_tag_from_media_no_wal(&mut self, media_id: &String, tag: &String) -> bool {
        let maybe_media = self.map.get_mut(media_id);
        if let Some(mut kvp) = maybe_media {
            let media = kvp.value_mut();
            if media.tags.contains(tag) {
                self.index.remove(media_id, media);
                media.tags.retain(|x| x != tag);
                self.index.insert(media_id, media);
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
        if self.map.contains_key(&hash) {
            return Ok(InsertResult::Existing(self.map.get(&hash).map(|x| x.value().clone()).unwrap()));
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
        if self.map.contains_key(&hash) {
            return Ok(InsertResult::Existing(self.map.get(&hash).map(|x| x.value().clone()).unwrap()));
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

    pub async fn delete_media(&mut self, media_id: &String) -> anyhow::Result<Option<Media>> {
        let maybe_media = self.delete_media_no_wal(media_id);
        if let Some(media) = &maybe_media {
            self.write_wal(DbOperation::DeleteMedia { media_id: media.id.clone() }).await?;
        }
        Ok(maybe_media)
    }

    pub async fn add_tag_to_media(&mut self, media_id: &String, tag: &String) -> anyhow::Result<bool> {
        let result = self.add_tag_to_media_no_wal(media_id, tag);
        if result {
            self.write_wal(DbOperation::AddTagToMedia { media_id: media_id.clone(), tag: tag.clone() }).await?;
        }
        Ok(result)
    }

    pub async fn remove_tag_from_media(&mut self, media_id: &String, tag: &String) -> anyhow::Result<bool> {
        let result = self.remove_tag_from_media_no_wal(media_id, tag);
        if result {
            self.write_wal(DbOperation::RemoveTagFromMedia { media_id: media_id.clone(), tag: tag.clone() }).await?;
        }
        Ok(result)
    }

    pub fn get_media_path(&self, media_id: &String) -> Option<PathBuf> {
        let media = self.get_media_by_id(media_id)?;
        let media_path = self.cfg.workdir.join(&media.location);
        Some(media_path)
    }

    pub fn get_thumbnail_path(&self, media_id: &String) -> PathBuf {
        let thumbnail_path = self.cfg.thumbnails_dir.join(format!("{}.png", media_id));
        thumbnail_path
    }
}
