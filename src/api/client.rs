use std::sync::Arc;
use axum_macros::FromRef;
use reqwest::Client;
use serde_json::json;
use crate::db::entities::{Media, Tag};

#[derive(Debug)]
struct ApiConfig {
    api_url: String,
}

#[derive(Clone, Debug, FromRef)]
pub struct ApiClient {
    client: Client,
    config: Arc<ApiConfig>,
}

impl ApiClient {
    pub fn new(api_url: String) -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(ApiConfig { api_url }),
        }
    }

    pub async fn add_media(&self, filepath: &str) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media", self.config.api_url);
        let res = self.client.post(&url).json(&json!({ "filename": filepath })).send().await?;
        Ok(res)
    }

    pub async fn search_tags(&self, query: &str) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/tags/search", self.config.api_url);
        let res = self.client.post(&url).json(&json!({ "q": query })).send().await?;
        Ok(res)
    }
}
