use std::str::FromStr;
use jammdb::DB;
use crate::db::entities::{Media, MediaId, Tag, TagId};

pub mod entities;
pub mod id;
pub mod hash;

pub enum Inserted<T> {
    New(T),
    AlreadyExists(T),
}

impl <T> Inserted<T> {
    pub fn unwrap(self) -> T {
        match self {
            Inserted::New(t) => t,
            Inserted::AlreadyExists(t) => t,
        }
    }
}

pub type DbResult<T> = Result<T, DbError>;

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("jammdb error: {0}")]
    EmbeddedDbError(#[from] jammdb::Error),
    #[error("utf8 error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("id error: {0}")]
    IdError(#[from] id::IdError),
    #[error("rmp-serde error: {0}")]
    SerializationError(#[from] rmp_serde::encode::Error),
    #[error("rmp-serde error: {0}")]
    DeserializationError(#[from] rmp_serde::decode::Error),
}

#[derive(Clone)]
pub struct DbRepo {
    db: DB,
}

impl DbRepo {
    pub fn new(db: DB) -> Self {
        Self {
            db,
        }
    }

    pub fn get_media_by_id(&self, id: MediaId) -> DbResult<Option<Media>> {
        let tx = self.db.tx(false)?;
        match tx.get_bucket("media") {
            Ok(media_bucket) => match media_bucket.get(id.to_string()) {
                Some(data) => {
                    let media_bytes = data.kv().value().to_owned();
                    let media = rmp_serde::from_slice(&media_bytes)?;
                    Ok(Some(media))
                },
                None => Ok(None),
            },
            Err(jammdb::Error::BucketMissing) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_media_by_hash(&self, hash: String) -> DbResult<Option<Media>> {
        let tx = self.db.tx(false)?;
        match tx.get_bucket("media_hash") {
            Ok(hash_bucket) => match hash_bucket.get(hash) {
                Some(data) => {
                    let media_id: String = std::str::from_utf8(data.kv().value())?.to_owned();
                    let media_id = MediaId::from_str(&media_id)?;
                    self.get_media_by_id(media_id)
                },
                None => Ok(None),
            },
            Err(jammdb::Error::BucketMissing) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_tag_by_id(&self, id: TagId) -> DbResult<Option<Tag>> {
        let tx = self.db.tx(false)?;
        match tx.get_bucket("tag") {
            Ok(tag_bucket) => match tag_bucket.get(id.to_string()) {
                Some(data) => {
                    let tag_bytes = data.kv().value().to_owned();
                    let tag = rmp_serde::from_slice(&tag_bytes)?;
                    Ok(Some(tag))
                },
                None => Ok(None),
            },
            Err(jammdb::Error::BucketMissing) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_tag_by_name(&self, name: String) -> DbResult<Option<Tag>> {
        let tx = self.db.tx(false)?;
        match tx.get_bucket("tag_name") {
            Ok(tag_name_bucket) => match tag_name_bucket.get(name) {
                Some(data) => {
                    let tag_id: String = std::str::from_utf8(data.kv().value())?.to_owned();
                    let tag_id = TagId::from_str(&tag_id)?;
                    self.get_tag_by_id(tag_id)
                },
                None => Ok(None),
            },
            Err(jammdb::Error::BucketMissing) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn search_tag_by_name(&self, name: String) -> DbResult<Vec<(Tag, usize)>> {
        let tx = self.db.tx(false)?;
        match tx.get_bucket("tag") {
            Ok(tag_bucket) => {
                let mut tags = Vec::new();
                for kv in tag_bucket.kv_pairs() {
                    let tag_bytes = kv.value();
                    let tag: Tag = rmp_serde::from_slice(tag_bytes)?;
                    let distance = levenshtein::levenshtein(&tag.name, &name);
                    if distance <= 1 {
                        tags.push((tag, distance));
                    } else if tag.name.contains(&name) {
                        tags.push((tag, 2));
                    }
                }
                tags.sort_by(|(_, d1), (_, d2)| d1.cmp(d2));
                Ok(tags)
            },
            Err(jammdb::Error::BucketMissing) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_or_insert_tag_by_name(&self, name: String) -> DbResult<Inserted<Tag>> {
        let maybe_tag = self.get_tag_by_name(name.clone())?;
        if let Some(tag) = maybe_tag {
            return Ok(Inserted::AlreadyExists(tag));
        }

        let tag = Tag {
            id: TagId::new(),
            name,
            created_at: chrono::Utc::now(),
            media: Vec::new(),
        };
        self.insert_tag(&tag)
    }

    pub fn get_all_tags(&self) -> DbResult<Vec<Tag>> {
        let tx = self.db.tx(false)?;
        match tx.get_bucket("tag") {
            Ok(tag_bucket) => {
                let mut tags = Vec::new();
                for kv in tag_bucket.kv_pairs() {
                    let tag_bytes = kv.value();
                    let tag: Tag = rmp_serde::from_slice(tag_bytes)?;
                    tags.push(tag);
                }
                Ok(tags)
            },
            Err(jammdb::Error::BucketMissing) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_all_media(&self) -> DbResult<Vec<Media>> {
        let tx = self.db.tx(false)?;
        match tx.get_bucket("media") {
            Ok(media_bucket) => {
                let mut media = Vec::new();
                for kv in media_bucket.kv_pairs() {
                    let media_bytes = kv.value();
                    let media_obj = rmp_serde::from_slice(media_bytes)?;
                    media.push(media_obj);
                }
                Ok(media)
            },
            Err(jammdb::Error::BucketMissing) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn insert_media(&self, media: &Media) -> DbResult<Inserted<Media>> {
        let media_id = media.id.to_string();
        let media_hash = media.hash.clone();

        let tx = self.db.tx(true)?;
        let hash_bucket = tx.get_or_create_bucket("media_hash")?;
        match hash_bucket.get(&media_hash) {
            None => {
                let media_bucket = tx.get_or_create_bucket("media")?;
                let media_bytes = rmp_serde::to_vec(media)?;
                media_bucket.put(media_id.clone(), media_bytes)?;
                hash_bucket.put(media_hash, media_id)?;
                tx.commit()?;
                Ok(Inserted::New(media.clone()))
            },
            Some(data) => Ok(Inserted::AlreadyExists(rmp_serde::from_slice(data.kv().value())?)),
        }
    }

    pub fn insert_tag(&self, tag: &Tag) -> DbResult<Inserted<Tag>> {
        let tag_id = tag.id.to_string();
        let tag_name = tag.name.clone();

        let tx = self.db.tx(true)?;
        let tag_name_bucket = tx.get_or_create_bucket("tag_name")?;
        match tag_name_bucket.get(&tag_name) {
            None => {
                let tag_bucket = tx.get_or_create_bucket("tag")?;
                let tag_bytes = rmp_serde::to_vec(tag)?;
                tag_bucket.put(tag_id.clone(), tag_bytes)?;
                tag_name_bucket.put(tag_name, tag_id)?;
                tx.commit()?;
                Ok(Inserted::New(tag.clone()))
            },
            Some(data) => Ok(Inserted::AlreadyExists(rmp_serde::from_slice(data.kv().value())?)),
        }
    }

    pub fn add_tag_to_media(&self, mut media: Media, mut tag: Tag) -> DbResult<Media> {
        if media.tags.contains(&tag.id) {
            return Ok(media);
        }

        let media_id = media.id.clone();
        let tag_id = tag.id.clone();

        media.tags.push(tag_id.clone());
        tag.media.push(media_id.clone());

        let tx = self.db.tx(true)?;

        let media_bucket = tx.get_or_create_bucket("media")?;
        let media_bytes = rmp_serde::to_vec(&media)?;
        media_bucket.put(media_id.to_string(), media_bytes)?;

        let tag_bucket = tx.get_or_create_bucket("tag")?;
        let tag_bytes = rmp_serde::to_vec(&tag)?;
        tag_bucket.put(tag_id.to_string(), tag_bytes)?;

        tx.commit()?;
        Ok(media)
    }

    pub fn delete_tag_from_media(&self, mut media: Media, mut tag: Tag) -> DbResult<Media> {
        if !media.tags.contains(&tag.id) {
            return Ok(media);
        }

        let media_id = media.id.clone();
        let tag_id = tag.id.clone();

        media.tags.push(tag_id.clone());
        tag.media.push(media_id.clone());

        let tx = self.db.tx(true)?;

        let media_bucket = tx.get_or_create_bucket("media")?;
        let media_bytes = rmp_serde::to_vec(&media)?;
        media_bucket.put(media_id.to_string(), media_bytes)?;

        let tag_bucket = tx.get_or_create_bucket("tag")?;
        let tag_bytes = rmp_serde::to_vec(&tag)?;
        tag_bucket.put(tag_id.to_string(), tag_bytes)?;

        tx.commit()?;
        Ok(media)
    }

    pub fn delete_media(&self, media: &Media) -> DbResult<()> {
        let tx = self.db.tx(true)?;

        let hash_bucket = tx.get_or_create_bucket("media_hash")?;
        if hash_bucket.get(media.hash.clone()).is_some() {
            hash_bucket.delete(media.hash.clone())?;
        }

        let media_bucket = tx.get_or_create_bucket("media")?;
        if media_bucket.get(media.id.to_string()).is_some() {
            media_bucket.delete(media.id.to_string())?;
        }

        let tag_bucket = tx.get_or_create_bucket("tag")?;
        for tag_id in media.tags.iter() {
            let maybe_tag_data = tag_bucket.get(tag_id.to_string());
            match maybe_tag_data {
                None => continue,
                Some(data) => {
                    let tag_bytes = data.kv().value().to_owned();
                    let mut tag: Tag = rmp_serde::from_slice(&tag_bytes)?;
                    tag.media.retain(|id| id !=  &media.id);
                    let tag_bytes = rmp_serde::to_vec(&tag)?;
                    tag_bucket.put(tag.id.to_string(), tag_bytes)?;
                },
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn delete_tag(&self, tag: &Tag) -> DbResult<()> {
        let tx = self.db.tx(true)?;

        let tag_name_bucket = tx.get_or_create_bucket("tag_name")?;
        if tag_name_bucket.get(tag.name.clone()).is_some() {
            tag_name_bucket.delete(tag.name.clone())?;
        }

        let tag_bucket = tx.get_or_create_bucket("tag")?;
        if tag_bucket.get(tag.id.to_string()).is_some() {
            tag_bucket.delete(tag.id.to_string())?;
        }

        let media_bucket = tx.get_or_create_bucket("media")?;
        for media_id in tag.media.iter() {
            let maybe_media_data = media_bucket.get(media_id.to_string());
            match maybe_media_data {
                None => continue,
                Some(data) => {
                    let media_bytes = data.kv().value().to_owned();
                    let mut media: Media = rmp_serde::from_slice(&media_bytes)?;
                    media.tags.retain(|id| id != &tag.id);
                    let media_bytes = rmp_serde::to_vec(&media)?;
                    media_bucket.put(media.id.to_string(), media_bytes)?;
                },
            }
        }

        tx.commit()?;
        Ok(())
    }
}
