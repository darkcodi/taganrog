use crate::api::client::ApiClient;

pub async fn add_media(api_url: &str, filepath: &str) {
    let api_client = ApiClient::new(api_url.to_string());
    let media = api_client.add_media(filepath).await.unwrap();
    println!("Added media: {:?}", media);
}