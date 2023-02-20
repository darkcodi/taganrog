use std::sync::Arc;
use anyhow::anyhow;
use serde::Deserialize;
use surrealdb::{Datastore, Response, Session};

#[derive(Clone)]
pub struct DbContext {
    store: Arc<Datastore>,
}

impl DbContext {
    pub fn new(db: Datastore) -> Self {
        Self {
            store: Arc::new(db),
        }
    }

    pub async fn exec(&self, query: &str) -> Result<Vec<Response>, surrealdb::Error>  {
        let session = Session::for_kv().with_ns("tg1").with_db("tg1");
        let db_response = self.store.execute(query, &session, None, true)
            .await?;
        Ok(db_response)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SurrealDbResult<T> {
    Ok {
        time: String,
        status: String,
        result: T,
    },
    Err {
        time: String,
        status: String,
        detail: String,
    },
}

pub trait SurrealStringify {
    fn surr_to_string(&self) -> anyhow::Result<String>;
}

impl SurrealStringify for Vec<Response> {
    fn surr_to_string(&self) -> anyhow::Result<String> {
        serde_json::to_string(self).map_err(anyhow::Error::new)
    }
}

pub trait RemoveFirst<T> {
    fn remove_first(&mut self) -> Option<T>;
}

impl<T> RemoveFirst<T> for Vec<T> {
    fn remove_first(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(self.remove(0))
    }
}

pub trait SurrealDeserializable {
    fn surr_deserialize<'a, T: Deserialize<'a>>(&'a self) -> anyhow::Result<T>;
}

impl SurrealDeserializable for String {
    fn surr_deserialize<'a, T: Deserialize<'a>>(&'a self) -> anyhow::Result<T> {
        let mut vec: Vec<SurrealDbResult<T>> = serde_json::from_str(self).map_err(anyhow::Error::new)?;
        let db_result = vec.remove_first().ok_or(anyhow!("Vec shouldn't be empty"))?;
        match db_result {
            SurrealDbResult::Ok { result, .. } => Ok(result),
            SurrealDbResult::Err { detail, .. } => Err(anyhow!(format!("Error from DB: {}", detail))),
        }
    }
}
