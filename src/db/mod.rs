use std::fmt::Formatter;
use anyhow::anyhow;
use reqwest::{Client, ClientBuilder};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    NetworkError(reqwest::Error),
    QueryError(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::NetworkError(e) => write!(f, "{e}"),
            DbError::QueryError(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Clone)]
pub struct DbContext {
    http_client: Client,
    url: String,
}

impl DbContext {
    pub fn new(base_url: String) -> Self {
        let mut header_map = HeaderMap::new();
        header_map.insert("Accept", HeaderValue::from_static("application/json"));
        header_map.insert("NS", HeaderValue::from_static("tg1"));
        header_map.insert("DB", HeaderValue::from_static("tg1"));
        let http_client = ClientBuilder::new()
            .default_headers(header_map)
            .use_rustls_tls()
            .build()
            .unwrap();
        let url = base_url + "/sql";
        Self {
            http_client,
            url,
        }
    }

    pub async fn exec(&self, query: &str) -> Result<String, DbError>  {
        let http_response = self.http_client.post(&self.url)
            .basic_auth("root", Some("root"))
            .body(query.to_string())
            .send()
            .await
            .map_err(DbError::NetworkError)?;
        let is_success = http_response.status().is_success();
        let db_response = http_response
            .text()
            .await
            .map_err(DbError::NetworkError)?;
        if is_success {
            Ok(db_response)
        } else {
            Err(DbError::QueryError(db_response))
        }
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
