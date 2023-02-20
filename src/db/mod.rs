use anyhow::anyhow;
use serde::Deserialize;
use surrealdb::Response;

#[derive(Deserialize)]
pub struct SurrealDbResult<T> {
    pub time: String,
    pub status: String,
    pub result: T,
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
        let result = db_result.result;
        Ok(result)
    }
}
