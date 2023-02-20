use chrono::{DateTime, Utc};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

// pub async fn ensure_exists(
//     name: &str,
//     db: &DatabaseConnection
// ) -> Result<Model, DbErr> {
//     let existing = Entity::find()
//         .filter(Column::Name.eq(name))
//         .one(db)
//         .await?;
//     if existing.is_some() {
//         return Ok(existing.unwrap());
//     }
//     let new = ActiveModel {
//         name: Set(name.to_string()),
//         ..Default::default()
//     }.insert(db).await?;
//     Ok(new)
// }
