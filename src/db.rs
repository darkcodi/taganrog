use chrono::{DateTime, Utc};
use dashmap::DashMap;
use indicium::simple::{AutocompleteType, Indexable, SearchIndex};
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};
use simple_wal::LogFile;
use tracing::info;
use crate::api::ApiContext;
use crate::utils::hash_utils::MurMurHasher;

pub enum DbInsertResult<T> {
    Existing(T),
    New(T),
}

impl<T> DbInsertResult<T> {
    pub fn safe_unwrap(self) -> T {
        match self {
            DbInsertResult::Existing(x) => x,
            DbInsertResult::New(x) => x,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TagWithCount {
    pub tag: String,
    pub count: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub size: i64,
    pub location: String,
    pub was_uploaded: bool,
    pub tags: Vec<String>,
}

impl Media {
    pub fn from_file(abs_path: &std::path::Path, rel_path: &RelativePath) -> anyhow::Result<Self> {
        let filename = abs_path.file_name().ok_or(anyhow::anyhow!("filename is empty"))?.to_string_lossy().to_string();
        let location = rel_path.to_string();
        let created_at = DateTime::from_naive_utc_and_offset(chrono::Utc::now().naive_utc(), Utc);
        let file_bytes = std::fs::read(&abs_path)?;
        let content_type = infer::get(&file_bytes).map(|x| x.mime_type()).unwrap_or("application/octet-stream").to_string();
        let hash = MurMurHasher::hash_bytes(&file_bytes);
        let metadata = std::fs::metadata(&abs_path)?;
        let size = metadata.len() as i64;

        Ok(Self {
            id: hash,
            filename,
            content_type,
            created_at,
            size,
            location,
            was_uploaded: false,
            tags: vec![],
        })
    }
}

impl Indexable for Media {
    fn strings(&self) -> Vec<String> {
        self.tags.clone()
    }
}

pub struct WalDb {
    map: DashMap<String, Media>,
    index: SearchIndex<String>,
    wal: LogFile,
}

impl WalDb {
    pub fn new(wal: LogFile) -> Self {
        Self {
            map: DashMap::new(),
            index: SearchIndex::default(),
            wal,
        }
    }

    fn get_media(&self, media_id: &String) -> Option<Media> {
        let maybe_media = self.map.get(media_id).map(|x| x.value().clone());
        maybe_media
    }

    fn get_all_media(&self, page_size: u64, page_index: u64) -> Vec<Media> {
        let media_vec = self.map.iter()
            .skip((page_index * page_size) as usize).take(page_size as usize)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    fn get_untagged_media(&self, page_size: u64, page_index: u64) -> Vec<Media> {
        let media_vec = self.map.iter()
            .filter(|x| x.value().tags.is_empty())
            .skip((page_index * page_size) as usize).take(page_size as usize)
            .map(|x| x.value().clone()).collect();
        media_vec
    }

    fn search_media(&self, tags: &[String], page_size: u64, page_index: u64) -> Vec<Media> {
        todo!()
    }

    fn search_tags(&self, tags: &[String], max_items: usize) -> Vec<TagWithCount> {
        let query = tags.join(" ");
        let tags = self.index.autocomplete_with(&AutocompleteType::Context, &max_items, &query);
        let tags = tags.into_iter().map(|x| TagWithCount { tag: x, count: 1 }).collect();
        tags
    }

    fn create_media(&mut self, media: Media) -> DbInsertResult<Media> {
        let id = media.id.clone();
        if self.map.contains_key(&id) {
            return DbInsertResult::Existing(media);
        }
        self.map.insert(id.clone(), media.clone());
        self.index.insert(&id, &media);
        DbInsertResult::New(media)
    }

    fn delete_media(&mut self, media_id: &String) -> Option<Media> {
        let maybe_media = self.map.remove(media_id);
        if let Some((id, media)) = maybe_media {
            self.index.remove(&id, &media);
            return Some(media);
        }
        None
    }

    fn add_tag_to_media(&mut self, media_id: &String, tag: &String) -> bool {
        let maybe_media = self.map.get_mut(media_id);
        if let Some(mut kvp) = maybe_media {
            let mut media = kvp.value_mut();
            if !media.tags.contains(tag) {
                media.tags.push(tag.clone());
                self.index.insert(&media_id, media);
                return true;
            }
        }
        false
    }

    fn remove_tag_from_media(&mut self, media_id: &String, tag: &String) -> bool {
        let maybe_media = self.map.get_mut(media_id);
        if let Some(mut kvp) = maybe_media {
            let mut media = kvp.value_mut();
            if media.tags.contains(tag) {
                self.index.remove(&media_id, media);
                media.tags.retain(|x| x != tag);
                self.index.insert(&media_id, media);
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum DbOperation {
    CreateMedia { media: Media },
    DeleteMedia { media_id: String },
    AddTagToMedia { media_id: String, tag: String },
    RemoveTagFromMedia { media_id: String, tag: String },
}

pub async fn init(ctx: &ApiContext) -> anyhow::Result<()> {
    info!("Starting DB import from WAL...");
    let mut db = ctx.db.write().await;
    let iter = db.wal.iter(..)?;
    let operations: Vec<_> = iter.collect::<Result<_, _>>()?;
    for item in operations {
        let operation: DbOperation = rmp_serde::from_slice(&*item)?;
        match operation {
            DbOperation::CreateMedia { media } => { db.create_media(media); }
            DbOperation::DeleteMedia { media_id } => { db.delete_media(&media_id); }
            DbOperation::AddTagToMedia { media_id, tag } => { db.add_tag_to_media(&media_id, &tag); }
            DbOperation::RemoveTagFromMedia { media_id, tag } => { db.remove_tag_from_media(&media_id, &tag); }
        }
    }
    info!("DB Imported!");
    Ok(())
}

pub async fn get_all_media(ctx: &ApiContext, page_size: u64, page_index: u64) -> anyhow::Result<Vec<Media>> {
    let db = ctx.db.read().await;
    let media_vec = db.get_all_media(page_size, page_index);
    Ok(media_vec)
}

pub async fn get_untagged_media(ctx: &ApiContext, page_size: u64, page_index: u64) -> anyhow::Result<Vec<Media>> {
    let db = ctx.db.read().await;
    let media_vec = db.get_untagged_media(page_size, page_index);
    Ok(media_vec)
}

pub async fn get_media_by_id(ctx: &ApiContext, media_id: &String) -> anyhow::Result<Option<Media>> {
    let db = ctx.db.read().await;
    let maybe_media = db.get_media(media_id);
    Ok(maybe_media)
}

pub async fn search_media(ctx: &ApiContext, tags: &[String], page_size: u64, page_index: u64) -> anyhow::Result<Vec<Media>> {
    todo!()
}

pub async fn search_tags(ctx: &ApiContext, tags: &[String], max_items: usize) -> anyhow::Result<Vec<TagWithCount>> {
    let db = ctx.db.read().await;
    let tags = db.search_tags(tags, max_items);
    Ok(tags)
}

pub async fn create_media(ctx: &ApiContext, media: Media) -> anyhow::Result<DbInsertResult<Media>> {
    let mut db = ctx.db.write().await;
    let result = db.create_media(media.clone());
    if let DbInsertResult::New(media) = &result {
        write_wal(&mut db, DbOperation::CreateMedia { media: media.clone() }).await?;
    }
    Ok(result)
}

pub async fn delete_media_by_id(ctx: &ApiContext, media_id: &String) -> anyhow::Result<Option<Media>> {
    let mut db = ctx.db.write().await;
    let maybe_media = db.delete_media(media_id);
    if let Some(media) = &maybe_media {
        write_wal(&mut db, DbOperation::DeleteMedia { media_id: media.id.clone() }).await?;
    }
    Ok(maybe_media)
}

pub async fn add_tag_to_media(ctx: &ApiContext, media_id: &String, tag: &String) -> anyhow::Result<bool> {
    let mut db = ctx.db.write().await;
    let result = db.add_tag_to_media(media_id, tag);
    if result {
        write_wal(&mut db, DbOperation::AddTagToMedia { media_id: media_id.clone(), tag: tag.clone() }).await?;
    }
    Ok(result)
}

pub async fn remove_tag_from_media(ctx: &ApiContext, media_id: &String, tag: &String) -> anyhow::Result<bool> {
    let mut db = ctx.db.write().await;
    let result = db.remove_tag_from_media(media_id, tag);
    if result {
        write_wal(&mut db, DbOperation::RemoveTagFromMedia { media_id: media_id.clone(), tag: tag.clone() }).await?;
    }
    Ok(result)
}

async fn write_wal(db: &mut WalDb, operation: DbOperation) -> anyhow::Result<()> {
    info!("Writing to WAL: {:?}", operation);
    let mut serialized_operation = rmp_serde::to_vec(&operation)?;
    db.wal.write(&mut serialized_operation)?;
    db.wal.flush()?;
    info!("WAL written!");
    Ok(())
}


