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

impl From<MediaWithTagsRows> for Option<MediaResponse> {
    fn from(value: MediaWithTagsRows) -> Self {
        if value.is_empty() {
            return None;
        }

        let mut tags_vec = Vec::with_capacity(value.0.len());
        for kvp in &value.0 {
            let tag = &kvp.1;
            if tag.is_some() {
                tags_vec.push(tag.as_ref().unwrap().name.clone());
            }
        }

        let media = value[0].0.clone();
        let mut media: MediaResponse = media.into();
        media.tags = tags_vec;
        Some(media)
    }
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
    let media: Option<MediaResponse> = media.into();
    Ok(media)
}

pub async fn find_by_hash(
    hash: &str,
    db: &DatabaseConnection,
) -> Result<Option<MediaResponse>, DbErr>  {
    let media: MediaWithTagsRows = Entity::find()
        .find_also_linked(media_tag::MediaToTag)
        .filter(Column::Hash.eq(hash))
        .all(db)
        .await?
        .into();
    let media: Option<MediaResponse> = media.into();
    Ok(media)
}
