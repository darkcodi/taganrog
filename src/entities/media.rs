use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use uuid::Uuid;
use amplify_derive::Wrapper;
use crate::entities::{media_tag, tag};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub guid: Uuid,
    pub original_filename: String,
    pub extension: Option<String>,
    pub new_filename: String,
    pub content_type: String,
    pub created_at: NaiveDateTime,
    pub hash: String,
    pub public_url: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MediaResponse {
    pub id: i64,
    pub guid: Uuid,
    pub original_filename: String,
    pub extension: Option<String>,
    pub new_filename: String,
    pub content_type: String,
    pub created_at: NaiveDateTime,
    pub hash: String,
    pub public_url: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Wrapper, Default, amplify_derive::From, Debug)]
struct MediaWithTagsRows(
    #[wrap]
    #[from]
    Vec<(Model, Option<tag::Model>)>
);

impl From<Model> for MediaResponse {
    fn from(value: Model) -> Self {
        Self {
            id: value.id,
            guid: value.guid,
            original_filename: value.original_filename,
            extension: value.extension,
            new_filename: value.new_filename,
            content_type: value.content_type,
            created_at: value.created_at,
            hash: value.hash,
            public_url: value.public_url,
            tags: Vec::new(),
        }
    }
}

impl From<MediaWithTagsRows> for Vec<MediaResponse> {
    fn from(value: MediaWithTagsRows) -> Self {
        if value.is_empty() {
            return Vec::new();
        }

        let grouping = value.0.group_by(|x, y| x.0.id == y.0.id);
        let mut result = Vec::new();
        for kvp in grouping {
            let mut model: MediaResponse = kvp[0].0.clone().into();
            let tags = kvp.iter()
                .filter(|x| x.1.is_some())
                .map(|x| x.1.as_ref().unwrap().name.clone())
                .collect();
            model.tags = tags;
            result.push(model);
        }
        result
    }
}

pub async fn find_all(
    page_size: u64,
    page_number: u64,
    db: &DatabaseConnection,
) -> Result<Vec<MediaResponse>, DbErr>  {
    let media: MediaWithTagsRows = Entity::find()
        .find_also_linked(media_tag::MediaToTag)
        .paginate(db, page_size)
        .fetch_page(page_number)
        .await?
        .into();
    let media: Vec<MediaResponse> = media.into();
    Ok(media)
}

pub async fn find_by_id(
    media_id: i64,
    db: &DatabaseConnection,
) -> Result<Option<MediaResponse>, DbErr>  {
    let media: MediaWithTagsRows = Entity::find_by_id(media_id)
        .find_also_linked(media_tag::MediaToTag)
        .all(db)
        .await?
        .into();
    let mut media: Vec<MediaResponse> = media.into();
    if media.is_empty() {
        return Ok(None);
    }
    let media = media.remove(0);
    Ok(Some(media))
}

pub async fn find_by_hash(
    hash: &str,
    db: &DatabaseConnection,
) -> Result<Option<MediaResponse>, DbErr>  {
    let media: MediaWithTagsRows = Entity::find()
        .filter(Column::Hash.eq(hash))
        .find_also_linked(media_tag::MediaToTag)
        .all(db)
        .await?
        .into();
    let mut media: Vec<MediaResponse> = media.into();
    if media.is_empty() {
        return Ok(None);
    }
    let media = media.remove(0);
    Ok(Some(media))
}
