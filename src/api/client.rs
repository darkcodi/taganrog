use std::sync::Arc;
use axum_macros::FromRef;
use reqwest::Client;
use serde_json::json;

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

    pub async fn get_media(&self, media_id: &str) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/{}", self.config.api_url, media_id);
        let res = self.client.get(&url).send().await?;
        Ok(res)
    }

    pub async fn autocomplete_tags(&self, query: &str, page: u64) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/tags/autocomplete", self.config.api_url);
        let res = self.client.post(&url).json(&json!({ "q": query, "p": page })).send().await?;
        Ok(res)
    }

    pub async fn search_media(&self, query: &str, page_size: u64, page_index: u64) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/search", self.config.api_url);
        let res = self.client.post(&url).json(&json!({ "q": query, "p": page_index, "s": page_size })).send().await?;
        Ok(res)
    }

    pub async fn stream_media(&self, media_id: &str) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/{}/stream", self.config.api_url, media_id);
        let res = self.client.get(&url).send().await?;
        Ok(res)
    }
}
