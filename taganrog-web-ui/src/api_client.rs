use std::sync::Arc;
use axum_macros::FromRef;
use clap::Parser;
use reqwest::Client;
use serde_json::json;
use taganrog_d::db::entities::Tag;

#[derive(Parser, Debug)]
pub struct ApiConfig {
    #[arg(env = "API_URL", default_value = "http://localhost:1698", help = "URL of the taganrog-d API")]
    pub api_url: String,
}

#[derive(Clone, Debug, FromRef)]
pub struct ApiClient {
    client: Client,
    config: Arc<ApiConfig>,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(ApiConfig::parse()),
        }
    }

    pub async fn search_tags(&self, query: &str) -> anyhow::Result<Vec<Tag>> {
        let url: String = format!("{}/api/tags/search", self.config.api_url);
        let res = self.client.post(&url).json(&json!({ "q": query })).send().await?;
        let tags: Vec<Tag> = res.json().await?;
        Ok(tags)
    }
}
