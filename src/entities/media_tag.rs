use sea_orm::entity::prelude::*;

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

impl ActiveModelBehavior for ActiveModel {}
