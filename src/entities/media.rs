use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub id: i64,
    pub guid: Uuid,
    pub original_filename: String,
    pub extension: Option<String>,
    pub new_filename: String,
    pub content_type: String,
    pub created_at: NaiveDateTime,
    pub hash: String,
    pub size: i64,
    pub public_url: String,
}

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
    pub size: i64,
    pub tags: Vec<String>,
}

impl From<Media> for MediaResponse {
    fn from(value: Media) -> Self {
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
            size: value.size,
            tags: Vec::new(),
        }
    }
}
// 
// pub async fn find_all(
//     page_size: u64,
//     page_number: u64,
//     db: &DatabaseConnection,
// ) -> Result<Vec<MediaResponse>, DbErr>  {
//     let media_ids: Vec<i64> = Entity::find()
//         .select_only()
//         .column(Column::Id)
//         .order_by_asc(Column::Id)
//         .into_tuple()
//         .paginate(db, page_size)
//         .fetch_page(page_number)
//         .await?;
//     if media_ids.is_empty() {
//         return Ok(Vec::new());
//     }
// 
//     let min_id = *media_ids.first().unwrap();
//     let max_id = *media_ids.last().unwrap();
//     let media: MediaWithTagsRows = Entity::find()
//         .filter(Column::Id.between(min_id, max_id))
//         .find_also_linked(media_tag::MediaToTag)
//         .all(db)
//         .await?
//         .into();
//     let media: Vec<MediaResponse> = media.into();
//     Ok(media)
// }
// 
// pub async fn search(
//     tags: &Vec<String>,
//     db: &DatabaseConnection,
// ) -> Result<Vec<MediaResponse>, DbErr>  {
//     let tag_ids: Vec<i64> = TagEntity::find()
//         .filter(TagColumn::Name.is_in(tags))
//         .select_only()
//         .column(TagColumn::Id)
//         .into_tuple()
//         .all(db)
//         .await?;
//     let media_ids: Vec<i64> = MediaTagEntity::find()
//         .filter(MediaTagColumn::TagId.is_in(tag_ids))
//         .select_only()
//         .column(MediaTagColumn::MediaId)
//         .into_tuple()
//         .all(db)
//         .await?;
//     let media: MediaWithTagsRows = Entity::find()
//         .filter(Column::Id.is_in(media_ids))
//         .find_also_linked(media_tag::MediaToTag)
//         .all(db)
//         .await?
//         .into();
//     let media: Vec<MediaResponse> = media.into();
//     Ok(media)
// }
// 
// pub async fn find_by_id(
//     media_id: i64,
//     db: &DatabaseConnection,
// ) -> Result<Option<MediaResponse>, DbErr>  {
//     let media: MediaWithTagsRows = Entity::find_by_id(media_id)
//         .find_also_linked(media_tag::MediaToTag)
//         .all(db)
//         .await?
//         .into();
//     let mut media: Vec<MediaResponse> = media.into();
//     if media.is_empty() {
//         return Ok(None);
//     }
//     let media = media.remove(0);
//     Ok(Some(media))
// }
// 
// pub async fn find_by_hash(
//     hash: &str,
//     db: &DatabaseConnection,
// ) -> Result<Option<MediaResponse>, DbErr>  {
//     let media: MediaWithTagsRows = Entity::find()
//         .filter(Column::Hash.eq(hash))
//         .find_also_linked(media_tag::MediaToTag)
//         .all(db)
//         .await?
//         .into();
//     let mut media: Vec<MediaResponse> = media.into();
//     if media.is_empty() {
//         return Ok(None);
//     }
//     let media = media.remove(0);
//     Ok(Some(media))
// }
