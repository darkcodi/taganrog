use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::sql::{Id, Thing};
use surrealdb::Surreal;
use tracing::debug;
use crate::db::entities::media::{Media, MediaId};
use crate::db::entities::tag::{Tag, TagId, TagWithCount};
use crate::db::executors::{DbExecutor, DbInsertResult};
use crate::utils::str_utils::StringExtensions;

type SurrealDbError = surrealdb::Error;
type SurrealDbResult<T> = Result<T, SurrealDbError>;

#[derive(Debug, Serialize)]
struct Document<T>
    where
        T: DeserializeOwned,
{
    #[serde(flatten)]
    pub inner: T,
    pub id: Thing,
}

impl<T> Document<T>
    where
        T: DeserializeOwned,
{
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Document<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Outer {
            id: Thing,
            #[serde(flatten)]
            inner: serde_json::Value,
        }

        let mut helper = Outer::deserialize(deserializer)?;
        let id = match &helper.id.id {
            Id::String(id_str) => id_str.to_string(),
            Id::Number(id_num) => id_num.to_string(),
            Id::Array(id_arr) => id_arr.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",").to_string(),
            Id::Object(id_obj) => serde_json::to_string(id_obj).unwrap(),
            Id::Generate(_) => "GEN".to_string(),
        };

        helper
            .inner
            .as_object_mut()
            .unwrap()
            .insert("id".to_string(), serde_json::to_value(id).unwrap());

        let inner: T = serde_json::from_value(helper.inner).unwrap();

        Ok(Document {
            inner,
            id: helper.id,
        })
    }
}

pub struct SurrealDbExecutor;

#[async_trait::async_trait]
impl DbExecutor for SurrealDbExecutor {
    type Db = Surreal<Db>;
    type DbError = SurrealDbError;

    async fn migrate(db: &Surreal<Db>) -> SurrealDbResult<()> {
        let query = "BEGIN TRANSACTION;
DEFINE TABLE tag SCHEMAFULL;
DEFINE FIELD name ON tag TYPE string ASSERT $value != NONE VALUE $value;
DEFINE INDEX tag_name ON TABLE tag COLUMNS name UNIQUE;
DEFINE FIELD created_at ON tag TYPE datetime VALUE $value OR time::now();
DEFINE ANALYZER autocomplete TOKENIZERS class FILTERS lowercase,ascii,edgengram(1,15);
DEFINE INDEX ix_name_search ON TABLE tag COLUMNS name SEARCH ANALYZER autocomplete BM25;
DEFINE TABLE media SCHEMAFULL;
DEFINE FIELD filename ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD content_type ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD created_at ON media TYPE datetime VALUE $value OR time::now();
DEFINE FIELD hash ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD size ON media TYPE int ASSERT $value != 0 VALUE $value;
DEFINE FIELD location ON media TYPE string ASSERT $value != NONE VALUE $value;
DEFINE FIELD was_uploaded ON media TYPE bool ASSERT $value != NONE VALUE $value;
DEFINE INDEX media_hash ON TABLE media COLUMNS hash UNIQUE;
DEFINE INDEX media_has_tag_uc ON TABLE has COLUMNS in, out UNIQUE;
COMMIT TRANSACTION;";
        debug!("DB Request: {query}");
        let mut response = db.query(query).await?;
        debug!("DB Response: {:?}", response);
        let errors = response.take_errors();
        if errors.len() > 0 {
            return Err(SurrealDbError::Db(surrealdb::err::Error::TxFailure));
        }
        Ok(())
    }

    async fn get_media_by_id(
        db: &Surreal<Db>,
        id: &MediaId,
    ) -> SurrealDbResult<Option<Media>> {
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media WHERE id = '{id}';");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let maybe_media: Option<Document<Media>> = response.take(0)?;
        Ok(maybe_media.map(|x| x.into_inner()))
    }

    async fn get_media_by_hash(
        db: &Surreal<Db>,
        hash: &str,
    ) -> SurrealDbResult<Option<Media>> {
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media WHERE hash = '{hash}';");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let maybe_media: Option<Document<Media>> = response.take(0)?;
        Ok(maybe_media.map(|x| x.into_inner()))
    }

    async fn get_all_media(
        db: &Surreal<Db>,
        page_size: u64,
        page_index: u64,
    ) -> SurrealDbResult<Vec<Media>> {
        let start = page_size * page_index;
        let limit = page_size;
        let query = format!("SELECT *, ->has->tag.name AS tags FROM media ORDER BY created_at LIMIT {limit} START {start};");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let media_vec: Vec<Document<Media>> = response.take(0)?;
        Ok(media_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    async fn get_untagged_media(
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Vec<Media>> {
        let query = "SELECT *, ->has->tag.name AS tags
FROM media
WHERE array::len(->has->tag) == 0;";
        debug!("DB Request: {query}");
        let mut response = db.query(query).await?;
        debug!("DB Response: {:?}", response);
        let media_vec: Vec<Document<Media>> = response.take(0)?;
        Ok(media_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    async fn search_media(
        db: &Surreal<Db>,
        tags: &[String],
    ) -> SurrealDbResult<Vec<Media>> {
        let tags_arr = tags.iter().map(|x| format!("'{}'", x.slugify())).join(", ");
        let query = format!("SELECT *, ->has->tag.name AS tags
FROM media
WHERE ->has->tag.name CONTAINSALL [{tags_arr}];");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let media_vec: Vec<Document<Media>> = response.take(0)?;
        Ok(media_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    async fn get_all_tags(
        db: &Surreal<Db>,
    ) -> SurrealDbResult<Vec<Tag>> {
        let query = "SELECT * FROM tag;";
        debug!("DB Request: {query}");
        let mut response = db.query(query).await?;
        debug!("DB Response: {:?}", response);
        let tag_vec: Vec<Document<Tag>> = response.take(0)?;
        Ok(tag_vec.into_iter().map(|x| x.into_inner()).collect())
    }

    async fn get_tag_by_id(
        db: &Surreal<Db>,
        id: &TagId,
    ) -> SurrealDbResult<Option<Tag>> {
        let query = format!("SELECT * FROM tag WHERE id = '{id}';");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let maybe_tag: Option<Document<Tag>> = response.take(0)?;
        Ok(maybe_tag.map(|x| x.into_inner()))
    }

    async fn get_tag_by_name(
        db: &Surreal<Db>,
        name: &str,
    ) -> SurrealDbResult<Option<Tag>> {
        let name = name.slugify();
        let query = format!("SELECT * FROM tag WHERE name = '{name}';");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let maybe_tag: Option<Document<Tag>> = response.take(0)?;
        Ok(maybe_tag.map(|x| x.into_inner()))
    }

    async fn search_tags(
        db: &Surreal<Db>,
        tags: &[String],
    ) -> SurrealDbResult<Vec<TagWithCount>> {
        let head = tags.iter().take(tags.len() - 1).map(|x| format!("'{}'", x.slugify())).join(",");
        let last = tags.last().unwrap().slugify();
        let query = format!("LET $head = [{head}];
LET $last = '{last}';

LET $last_tags = SELECT VALUE name FROM tag WHERE name @1@ $last;

SELECT * FROM
(
    SELECT id, name, created_at, count(id) AS count
    FROM array::flatten((
        SELECT VALUE ->has->tag.* AS rel_tags
        FROM media
        WHERE ->has->tag.name CONTAINSALL $head
        AND (->has->tag.name CONTAINSANY $last_tags OR string::len($last) == 0)
    ))
    WHERE $head CONTAINSNOT name AND string::startsWith(name, $last)
    GROUP BY id, name, created_at
)
ORDER BY count DESC LIMIT 10;");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let tag_vec: Vec<Document<TagWithCount>> = response.take(3)?;
        let teg_vec = tag_vec.into_iter().map(|x| x.into_inner()).collect();
        Ok(teg_vec)
    }

    async fn create_media(
        db: &Surreal<Db>,
        media: &Media,
    ) -> SurrealDbResult<DbInsertResult<Media>> {
        let media_id = &media.id.to_string();
        let filename = &media.filename;
        let content_type = &media.content_type;
        let created_at = &media.created_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(); // ISO-8601
        let hash = &media.hash;
        let location = &media.location;
        let size = &media.size;
        let was_uploaded = &media.was_uploaded;

        let query = format!("CREATE {media_id}
SET filename = '{filename}',
content_type = '{content_type}',
created_at = '{created_at}',
hash = '{hash}',
size = {size},
location = '{location}',
was_uploaded = {was_uploaded};");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let response_result = response.check();
        if let Err(surrealdb::Error::Db(surrealdb::err::Error::IndexExists { .. })) = response_result {
            let existing_media = Self::get_media_by_hash(db, hash).await?.unwrap();
            return Ok(DbInsertResult::Existing(existing_media));
        }
        let maybe_media: Option<Document<Media>> = response_result?.take(0)?;
        let mut media = maybe_media.unwrap().into_inner();
        if let None = media.tags {
            media.tags = Some(vec![]);
        }
        Ok(DbInsertResult::New(media))
    }

    async fn create_tag(
        db: &Surreal<Db>,
        name: &Tag,
    ) -> SurrealDbResult<DbInsertResult<Tag>> {
        let tag_id = &name.id.to_string();
        let name = &name.name;
        let query = format!("CREATE {tag_id}
SET name = '{name}',
created_at = time::now();");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let response_result = response.check();
        if let Err(surrealdb::Error::Db(surrealdb::err::Error::IndexExists { .. })) = response_result {
            let existing_tag = Self::get_tag_by_name(db, name).await?.unwrap();
            return Ok(DbInsertResult::Existing(existing_tag));
        }
        let maybe_tag: Option<Document<Tag>> = response_result?.take(0)?;
        let tag = maybe_tag.unwrap().into_inner();
        Ok(DbInsertResult::New(tag))
    }

    async fn delete_media_by_id(
        db: &Surreal<Db>,
        id: &MediaId,
    ) -> SurrealDbResult<Option<Media>> {
        let query = format!("DELETE FROM media WHERE id = '{id}' RETURN BEFORE;");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let maybe_media: Option<Document<Media>> = response.take(0)?;
        Ok(maybe_media.map(|x| x.into_inner()))
    }

    async fn delete_tag_by_id(
        db: &Surreal<Db>,
        id: &TagId,
    ) -> SurrealDbResult<Option<Tag>> {
        let query = format!("DELETE FROM tag WHERE id = '{id}' RETURN BEFORE;");
        debug!("DB Request: {query}");
        let mut response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        let maybe_tag: Option<Document<Tag>> = response.take(0)?;
        Ok(maybe_tag.map(|x| x.into_inner()))
    }

    async fn add_tag_to_media(
        db: &Surreal<Db>,
        media_id: &MediaId,
        tag_id: &TagId,
    ) -> SurrealDbResult<()> {
        let query = format!("RELATE {media_id}->has->{tag_id} SET time.added = time::now();");
        debug!("DB Request: {query}");
        let response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        Ok(())
    }

    async fn remove_tag_from_media(
        db: &Surreal<Db>,
        media_id: &MediaId,
        tag_id: &TagId,
    ) -> SurrealDbResult<()> {
        let query = format!("DELETE FROM has WHERE in = {media_id} and out.id = '{tag_id}';");
        debug!("DB Request: {query}");
        let response = db.query(query.as_str()).await?;
        debug!("DB Response: {:?}", response);
        Ok(())
    }
}
