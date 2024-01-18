use std::str::FromStr;
use jammdb::DB;
use crate::db::entities::{Media, MediaId, Tag, TagId};

pub mod entities;
pub mod id;

#[derive(Clone)]
pub struct DbRepo {
    db: DB,
}

pub enum InsertResult<T> {
    Inserted(T),
    AlreadyExists(T),
}

impl DbRepo {
    pub fn new(db: DB) -> Self {
        Self {
            db,
        }
    }

    pub fn get_media_by_id(&self, id: MediaId) -> anyhow::Result<Option<Media>> {
        let tx = self.db.tx(false)?;
        let media_bucket_result = tx.get_bucket("media");
        if let Err(jammdb::Error::BucketMissing) = media_bucket_result {
            return Ok(None);
        }
        let media_bucket = media_bucket_result?;
        let maybe_media_bytes = media_bucket.get(id.to_string());
        if maybe_media_bytes.is_none() {
            return Ok(None);
        }

        let media_bytes = maybe_media_bytes.unwrap().kv().value().to_owned();
        let media = rmp_serde::from_slice(&media_bytes).unwrap();
        Ok(Some(media))
    }

    pub fn get_media_by_hash(&self, hash: String) -> anyhow::Result<Option<Media>> {
        let tx = self.db.tx(false)?;
        let hash_bucket_result = tx.get_bucket("media_hash");
        if let Err(jammdb::Error::BucketMissing) = hash_bucket_result {
            return Ok(None);
        }
        let hash_bucket = hash_bucket_result?;
        let maybe_media_id = hash_bucket.get(hash);
        if maybe_media_id.is_none() {
            return Ok(None);
        }

        let media_id: String = std::str::from_utf8(maybe_media_id.unwrap().kv().value())?.to_owned();
        let media_id = MediaId::from_str(&media_id)?;
        self.get_media_by_id(media_id)
    }

    pub fn get_tag_by_id(&self, id: TagId) -> anyhow::Result<Option<Tag>> {
        let tx = self.db.tx(false)?;
        let tag_bucket_result = tx.get_bucket("tag");
        if let Err(jammdb::Error::BucketMissing) = tag_bucket_result {
            return Ok(None);
        }
        let tag_bucket = tag_bucket_result?;
        let maybe_tag_bytes = tag_bucket.get(id.to_string());
        if maybe_tag_bytes.is_none() {
            return Ok(None);
        }

        let tag_bytes = maybe_tag_bytes.unwrap().kv().value().to_owned();
        let tag = rmp_serde::from_slice(&tag_bytes).unwrap();
        Ok(Some(tag))
    }

    pub fn get_tag_by_name(&self, name: String) -> anyhow::Result<Option<Tag>> {
        let tx = self.db.tx(false)?;
        let tag_name_bucket_result = tx.get_bucket("tag_name");
        if let Err(jammdb::Error::BucketMissing) = tag_name_bucket_result {
            return Ok(None);
        }
        let tag_name_bucket = tag_name_bucket_result?;
        let maybe_tag_id = tag_name_bucket.get(name);
        if maybe_tag_id.is_none() {
            return Ok(None);
        }

        let tag_id: String = std::str::from_utf8(maybe_tag_id.unwrap().kv().value())?.to_owned();
        let tag_id = TagId::from_str(&tag_id)?;
        self.get_tag_by_id(tag_id)
    }

    pub fn get_all_media(&self) -> anyhow::Result<Vec<Media>> {
        let tx = self.db.tx(false)?;
        let media_bucket_result = tx.get_bucket("media");
        if let Err(jammdb::Error::BucketMissing) = media_bucket_result {
            return Ok(Vec::new());
        }
        let media_bucket = media_bucket_result?;
        let mut media = Vec::new();
        for kv in media_bucket.kv_pairs() {
            let media_bytes = kv.value();
            let media_obj = rmp_serde::from_slice(media_bytes).unwrap();
            media.push(media_obj);
        }
        Ok(media)
    }

    pub fn insert_media(&self, media: &Media) -> anyhow::Result<InsertResult<Media>> {
        let tx = self.db.tx(true)?;

        let media_id = media.id.to_string();
        let media_hash = media.hash.clone();
        let hash_bucket = tx.get_or_create_bucket("media_hash")?;
        let maybe_existing_media = hash_bucket.get(&media_hash);
        if maybe_existing_media.is_some() {
            let existing_media = rmp_serde::from_slice(maybe_existing_media.unwrap().kv().value()).unwrap();
            return Ok(InsertResult::AlreadyExists(existing_media));
        }

        let media_bucket = tx.get_or_create_bucket("media")?;
        let media_bytes = rmp_serde::to_vec(media).unwrap();
        media_bucket.put(media_id.clone(), media_bytes)?;
        hash_bucket.put(media_hash, media_id)?;

        tx.commit()?;
        Ok(InsertResult::Inserted(media.clone()))
    }

    pub fn insert_tag(&self, tag: &Tag) -> anyhow::Result<InsertResult<Tag>> {
        let tx = self.db.tx(true)?;

        let tag_id = tag.id.to_string();
        let tag_name = tag.name.clone();
        let tag_name_bucket = tx.get_or_create_bucket("tag_name")?;
        let maybe_existing_tag = tag_name_bucket.get(&tag_name);
        if maybe_existing_tag.is_some() {
            let existing_tag = rmp_serde::from_slice(maybe_existing_tag.unwrap().kv().value()).unwrap();
            return Ok(InsertResult::AlreadyExists(existing_tag));
        }

        let tag_bucket = tx.get_or_create_bucket("tag")?;
        let tag_bytes = rmp_serde::to_vec(tag).unwrap();
        tag_bucket.put(tag_id.clone(), tag_bytes)?;
        tag_name_bucket.put(tag_name, tag_id)?;

        tx.commit()?;
        Ok(InsertResult::Inserted(tag.clone()))
    }

    pub fn add_tag_to_media(&self, media_id: MediaId, tag_id: TagId) -> anyhow::Result<Media> {
        let maybe_media = self.get_media_by_id(media_id.clone())?;
        if maybe_media.is_none() {
            return Err(anyhow::anyhow!("media not found"));
        }
        let mut media = maybe_media.unwrap();

        let maybe_tag = self.get_tag_by_id(tag_id.clone())?;
        if maybe_tag.is_none() {
            return Err(anyhow::anyhow!("tag not found"));
        }
        let mut tag = maybe_tag.unwrap();

        media.tags.push(tag_id.clone());
        tag.media.push(media_id.clone());

        let tx = self.db.tx(true)?;

        let media_bucket = tx.get_or_create_bucket("media")?;
        let media_bytes = rmp_serde::to_vec(&media).unwrap();
        media_bucket.put(media_id.to_string(), media_bytes)?;

        let tag_bucket = tx.get_or_create_bucket("tag")?;
        let tag_bytes = rmp_serde::to_vec(&tag).unwrap();
        tag_bucket.put(tag_id.to_string(), tag_bytes)?;

        tx.commit()?;
        Ok(media)
    }

    pub fn remove_tag_from_media(&self, media_id: MediaId, tag_id: TagId) -> anyhow::Result<Media> {
        let maybe_media = self.get_media_by_id(media_id.clone())?;
        if maybe_media.is_none() {
            return Err(anyhow::anyhow!("media not found"));
        }
        let mut media = maybe_media.unwrap();

        let maybe_tag = self.get_tag_by_id(tag_id.clone())?;
        if maybe_tag.is_none() {
            return Err(anyhow::anyhow!("tag not found"));
        }
        let mut tag = maybe_tag.unwrap();

        media.tags.retain(|x| x != &tag_id);
        tag.media.retain(|x| x != &media_id);

        let tx = self.db.tx(true)?;

        let media_bucket = tx.get_or_create_bucket("media")?;
        let media_bytes = rmp_serde::to_vec(&media).unwrap();
        media_bucket.put(media_id.to_string(), media_bytes)?;

        let tag_bucket = tx.get_or_create_bucket("tag")?;
        let tag_bytes = rmp_serde::to_vec(&tag).unwrap();
        tag_bucket.put(tag_id.to_string(), tag_bytes)?;

        tx.commit()?;
        Ok(media)
    }

    pub fn delete_media(&self, media: &Media) -> anyhow::Result<()> {
        let tx = self.db.tx(true)?;

        let hash_bucket = tx.get_or_create_bucket("media_hash")?;
        if hash_bucket.get(media.hash.clone()).is_some() {
            hash_bucket.delete(media.hash.clone())?;
        }

        let media_bucket = tx.get_or_create_bucket("media")?;
        if media_bucket.get(media.id.to_string()).is_some() {
            media_bucket.delete(media.id.to_string())?;
        }

        tx.commit()?;
        Ok(())
    }
}
