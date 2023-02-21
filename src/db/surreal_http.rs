use std::fmt::Formatter;
use http_auth_basic::Credentials;
use reqwest::{Client, ClientBuilder};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use crate::utils::vec_utils::RemoveExtensions;

#[derive(Debug, thiserror::Error)]
pub enum SurrealDbError {
    NetworkError(reqwest::Error),
    DeserializationError(serde_json::Error),
    EmptyResponse,
    QueryExecutionError(String),
    NonSuccessfulStatusCode(HttpErrorDescription),
}

impl std::fmt::Display for SurrealDbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SurrealDbError::NetworkError(e) => write!(f, "network error: {e}"),
            SurrealDbError::DeserializationError(e) => write!(f, "deserialization error: {e}"),
            SurrealDbError::EmptyResponse => write!(f, "Response is empty"),
            SurrealDbError::QueryExecutionError(e) => write!(f, "query execution error: {e}"),
            SurrealDbError::NonSuccessfulStatusCode(e) => write!(f, "non-OK response: {}", serde_json::to_string(e).unwrap()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SurrealDbResponse {
    Ok(Vec<SurrealDbResult>),
    Err(HttpErrorDescription),
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

impl SurrealDbResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, SurrealDbResult::Ok { .. })
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpErrorDescription {
    pub code: u16,
    pub details: String,
    pub description: String,
    pub information: String,
}

#[derive(Clone)]
pub struct SurrealHttpClient {
    http_client: Client,
    url: String,
}

impl SurrealHttpClient {
    pub fn new(base_url: &str, user: &str, pass: &str, ns: &str, db: &str) -> Self {
        let auth = Credentials::new(user, pass).as_http_header();
        let mut auth = HeaderValue::from_str(auth.as_str()).unwrap();
        auth.set_sensitive(true);

        let mut header_map = HeaderMap::new();
        header_map.insert("Authorization", auth);
        header_map.insert("Accept", HeaderValue::from_static("application/json"));
        header_map.insert("NS", HeaderValue::from_str(ns).unwrap());
        header_map.insert("DB", HeaderValue::from_str(db).unwrap());

        let http_client = ClientBuilder::new()
            .default_headers(header_map)
            .use_rustls_tls()
            .build()
            .unwrap();
        let url = base_url.to_string() + "/sql";

        Self {
            http_client,
            url,
        }
    }

    pub async fn exec(&self, query: &str) -> Result<Vec<SurrealDbResult>, SurrealDbError>  {
        let http_response = self.http_client.post(&self.url)
            .body(query.to_string())
            .send()
            .await
            .map_err(SurrealDbError::NetworkError)?;
        let response_str = http_response
            .text()
            .await
            .map_err(SurrealDbError::NetworkError)?;
        let db_response: SurrealDbResponse = serde_json::from_str(&response_str)
            .map_err(SurrealDbError::DeserializationError)?;
        match db_response {
            SurrealDbResponse::Ok(res_vec) => Ok(res_vec),
            SurrealDbResponse::Err(e) => Err(SurrealDbError::NonSuccessfulStatusCode(e)),
        }
    }
}

pub trait SurrealDeserializable {
    fn surr_deserialize<T: DeserializeOwned>(self) -> Result<T, SurrealDbError>;
}

impl SurrealDeserializable for SurrealDbResult {
    fn surr_deserialize<T: DeserializeOwned>(self) -> Result<T, SurrealDbError> {
        match self {
            SurrealDbResult::Ok { result, .. } => {
                let entity: T = serde_json::from_value(result)
                    .map_err(SurrealDbError::DeserializationError)?;
                Ok(entity)
            },
            SurrealDbResult::Err { detail, .. } => Err(SurrealDbError::QueryExecutionError(detail.clone())),
        }
    }
}

pub trait SurrealVecDeserializable {
    fn surr_deserialize_first<T: DeserializeOwned>(self) -> Result<T, SurrealDbError>;
    fn surr_deserialize_last<T: DeserializeOwned>(self) -> Result<T, SurrealDbError>;
}

impl SurrealVecDeserializable for Vec<SurrealDbResult> {
    fn surr_deserialize_first<T: DeserializeOwned>(mut self) -> Result<T, SurrealDbError> {
        let db_result = self.remove_first().ok_or(SurrealDbError::EmptyResponse)?;
        db_result.surr_deserialize()
    }

    fn surr_deserialize_last<T: DeserializeOwned>(mut self) -> Result<T, SurrealDbError> {
        let db_result = self.remove_last().ok_or(SurrealDbError::EmptyResponse)?;
        db_result.surr_deserialize()
    }
}
