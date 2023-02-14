use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use uuid::Uuid;
use amplify_derive::Wrapper;
use sea_orm::{QueryOrder, QuerySelect};
use crate::entities::{media_tag, MediaTagColumn, MediaTagEntity, tag, TagColumn, TagEntity};

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
    fn from(mut value: MediaWithTagsRows) -> Self {
        if value.is_empty() {
            return Vec::new();
        }

        value.sort_by(|x, y| x.0.id.cmp(&y.0.id));

        let mut result: Vec<MediaResponse> = Vec::new();
        let mut current_index: Option<usize> = None;
        for kvp in value.iter() {
            let tag_name = kvp.1.clone().unwrap().name;
            if current_index.is_some() {
                let current_item: &mut _ = result.get_mut(current_index.unwrap()).unwrap();
                if current_item.id == kvp.0.id {
                    current_item.tags.push(tag_name);
                    continue;
                }
            }

            let mut new_item: MediaResponse = kvp.0.clone().into();
            new_item.tags.push(tag_name);
            result.push(new_item);
            current_index = Some(current_index.map(|x| x + 1).unwrap_or(0));
        }
        result
    }
}

pub async fn find_all(
    page_size: u64,
    page_number: u64,
    db: &DatabaseConnection,
) -> Result<Vec<MediaResponse>, DbErr>  {
    let media_ids: Vec<i64> = Entity::find()
        .select_only()
        .column(Column::Id)
        .order_by_asc(Column::Id)
        .into_tuple()
        .paginate(db, page_size)
        .fetch_page(page_number)
        .await?;
    if media_ids.is_empty() {
        return Ok(Vec::new());
    }

    let min_id = *media_ids.first().unwrap();
    let max_id = *media_ids.last().unwrap();
    let media: MediaWithTagsRows = Entity::find()
        .filter(Column::Id.between(min_id, max_id))
        .find_also_linked(media_tag::MediaToTag)
        .all(db)
        .await?
        .into();
    let media: Vec<MediaResponse> = media.into();
    Ok(media)
}

pub async fn search(
    tags: &Vec<String>,
    db: &DatabaseConnection,
) -> Result<Vec<MediaResponse>, DbErr>  {
    let tag_ids: Vec<i64> = TagEntity::find()
        .filter(TagColumn::Name.is_in(tags))
        .select_only()
        .column(TagColumn::Id)
        .into_tuple()
        .all(db)
        .await?;
    let media_ids: Vec<i64> = MediaTagEntity::find()
        .filter(MediaTagColumn::TagId.is_in(tag_ids))
        .select_only()
        .column(MediaTagColumn::MediaId)
        .into_tuple()
        .all(db)
        .await?;
    let media: MediaWithTagsRows = Entity::find()
        .filter(Column::Id.is_in(media_ids))
        .find_also_linked(media_tag::MediaToTag)
        .all(db)
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
