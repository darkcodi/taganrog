use sea_orm::entity::prelude::*;
use sea_orm::Set;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub media_id: i64,
    #[sea_orm(primary_key)]
    pub tag_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Media,
    Tag,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Media => Entity::belongs_to(super::media::Entity)
                .from(Column::MediaId)
                .to(super::media::Column::Id)
                .into(),
            Self::Tag => Entity::belongs_to(super::tag::Entity)
                .from(Column::TagId)
                .to(super::tag::Column::Id)
                .into(),
        }
    }
}

#[derive(Debug)]
pub struct MediaToTag;

#[derive(Debug)]
pub struct TagToMedia;

impl Linked for MediaToTag {
    type FromEntity = super::media::Entity;
    type ToEntity = super::tag::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            Relation::Media.def().rev(),
            Relation::Tag.def(),
        ]
    }
}

impl Linked for TagToMedia {
    type FromEntity = super::tag::Entity;
    type ToEntity = super::media::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            Relation::Tag.def().rev(),
            Relation::Media.def(),
        ]
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn ensure_exists(
    media_id: i64,
    tag_id: i64,
    db: &DatabaseConnection
) -> Result<Model, DbErr> {
    let existing = Entity::find_by_id((media_id, tag_id))
        .one(db)
        .await?;
    if existing.is_some() {
        return Ok(existing.unwrap());
    }
    let new = ActiveModel {
        media_id: Set(media_id),
        tag_id: Set(tag_id),
    }.insert(db).await?;
    Ok(new)
}

pub async fn ensure_not_exists(
    media_id: i64,
    tag_id: i64,
    db: &DatabaseConnection
) -> Result<(), DbErr> {
    let existing = Entity::find_by_id((media_id, tag_id))
        .one(db)
        .await?;
    if existing.is_some() {
        existing.unwrap().delete(db).await?;
    }
    Ok(())
}
