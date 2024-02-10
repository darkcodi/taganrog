use std::sync::Arc;
use axum_macros::FromRef;
use reqwest::Client;
use serde::Serialize;
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

#[derive(Default, Debug, Serialize)]
pub struct MultipartFile {
    pub file_name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub preview_file_name: String,
    pub preview_content_type: String,
    pub preview_bytes: Vec<u8>,
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

    pub async fn get_random_media(&self) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/random", self.config.api_url);
        let res = self.client.get(&url).send().await?;
        Ok(res)
    }

    pub async fn delete_media(&self, media_id: &str) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/{}", self.config.api_url, media_id);
        let res = self.client.delete(&url).send().await?;
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

    pub async fn add_tag_to_media(&self, media_id: &str, tag: &str) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/{}/add-tag", self.config.api_url, media_id);
        let res = self.client.post(&url).json(&json!({ "name": tag })).send().await?;
        Ok(res)
    }

    pub async fn delete_tag_from_media(&self, media_id: &str, tag: &str) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/{}/remove-tag", self.config.api_url, media_id);
        let res = self.client.post(&url).json(&json!({ "name": tag })).send().await?;
        Ok(res)
    }

    pub async fn upload_media(&self, file: MultipartFile) -> anyhow::Result<reqwest::Response> {
        let url: String = format!("{}/api/media/upload", self.config.api_url);
        let res = self.client.post(&url).multipart(reqwest::multipart::Form::new()
            .part("file",
                  reqwest::multipart::Part::bytes(file.bytes)
                      .file_name(file.file_name)
                      .mime_str(&file.content_type)?)
            .part("preview",
                  reqwest::multipart::Part::bytes(file.preview_bytes)
                      .file_name(file.preview_file_name)
                      .mime_str(&file.preview_content_type)?))
            .send().await?;
        Ok(res)
    }
}
