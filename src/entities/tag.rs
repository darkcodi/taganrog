use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use sea_orm::Set;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn ensure_exists(
    name: &str,
    db: &DatabaseConnection
) -> Result<Model, DbErr> {
    let existing = Entity::find()
        .filter(Column::Name.eq(name))
        .one(db)
        .await?;
    if existing.is_some() {
        return Ok(existing.unwrap());
    }
    let new = ActiveModel {
        name: Set(name.to_string()),
        ..Default::default()
    }.insert(db).await?;
    Ok(new)
}
