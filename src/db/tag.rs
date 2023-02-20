use chrono::{DateTime, Utc};
use crate::db::surreal_http::{SurrealDbError, SurrealDeserializable, SurrealHttpClient};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl Tag {
    pub async fn get_all(
        db: &SurrealHttpClient,
    ) -> Result<Vec<Tag>, SurrealDbError> {
        let result: Vec<Tag> = db.exec("SELECT * FROM tag;")
            .await?
            .surr_deserialize()?;
        Ok(result)
    }
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
