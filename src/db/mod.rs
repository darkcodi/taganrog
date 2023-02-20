use std::fmt::Formatter;
use reqwest::{Client, ClientBuilder};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    NetworkError(reqwest::Error),
    DeserializationError(serde_json::Error),
    EmptyResponse,
    QueryExecutionError(String),
    NonSuccessfulStatusCode(SurrealDbError),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::NetworkError(e) => write!(f, "network error: {e}"),
            DbError::DeserializationError(e) => write!(f, "deserialization error: {e}"),
            DbError::EmptyResponse => write!(f, "Response is empty"),
            DbError::QueryExecutionError(e) => write!(f, "query execution error: {e}"),
            DbError::NonSuccessfulStatusCode(e) => write!(f, "non-OK response: {}", serde_json::to_string(e).unwrap()),
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

    pub async fn exec(&self, query: &str) -> Result<Vec<SurrealDbResult>, DbError>  {
        let http_response = self.http_client.post(&self.url)
            .basic_auth("root", Some("root"))
            .body(query.to_string())
            .send()
            .await
            .map_err(DbError::NetworkError)?;
        let response_str = http_response
            .text()
            .await
            .map_err(DbError::NetworkError)?;
        let db_response: SurrealDbResponse = serde_json::from_str(&response_str)
            .map_err(DbError::DeserializationError)?;
        match db_response {
            SurrealDbResponse::Ok(res_vec) => Ok(res_vec),
            SurrealDbResponse::Err(e) => Err(DbError::NonSuccessfulStatusCode(e)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SurrealDbResponse {
    Ok(Vec<SurrealDbResult>),
    Err(SurrealDbError),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SurrealDbResult {
    Ok {
        time: String,
        status: String,
        result: serde_json::Value,
    },
    Err {
        time: String,
        status: String,
        detail: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SurrealDbError {
    pub code: u16,
    pub details: String,
    pub description: String,
    pub information: String,
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
    fn surr_deserialize<T: DeserializeOwned>(self) -> Result<T, DbError>;
}

impl SurrealDeserializable for SurrealDbResult {
    fn surr_deserialize<T: DeserializeOwned>(self) -> Result<T, DbError> {
        match self {
            SurrealDbResult::Ok { result, .. } => {
                let entity: T = serde_json::from_value(result)
                    .map_err(DbError::DeserializationError)?;
                Ok(entity)
            },
            SurrealDbResult::Err { detail, .. } => Err(DbError::QueryExecutionError(detail.clone())),
        }
    }
}

impl SurrealDeserializable for Vec<SurrealDbResult> {
    fn surr_deserialize<T: DeserializeOwned>(mut self) -> Result<T, DbError> {
        let first_result = self.remove_first().ok_or(DbError::EmptyResponse)?;
        first_result.surr_deserialize()
    }
}
