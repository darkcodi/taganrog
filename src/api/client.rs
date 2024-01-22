use std::sync::Arc;
use axum_macros::FromRef;
use reqwest::Client;
use serde_json::json;
use crate::db::entities::Tag;

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

    pub async fn search_tags(&self, query: &str) -> anyhow::Result<Vec<Tag>> {
        let url: String = format!("{}/api/tags/search", self.config.api_url);
        let res = self.client.post(&url).json(&json!({ "q": query })).send().await?;
        let tags: Vec<Tag> = res.json().await?;
        Ok(tags)
    }
}
