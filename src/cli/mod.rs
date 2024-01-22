use crate::api::client::ApiClient;
use crate::db::entities::Media;

pub async fn add_media(api_url: &str, filepath: &str) {
    let canonical_filepath_result = std::fs::canonicalize(filepath);
    if canonical_filepath_result.is_err() {
        let err = canonical_filepath_result.err().unwrap();
        if err.kind() == std::io::ErrorKind::NotFound {
            eprintln!("File not found: {}", filepath);
            std::process::exit(1);
        } else {
            eprintln!("IO Error: {}", err);
            std::process::exit(1);
        }
    }

    let canonical_filepath = canonical_filepath_result.unwrap();
    println!("File: {:?}", canonical_filepath);
    let api_client = ApiClient::new(api_url.to_string());
    let api_response_result = api_client.add_media(canonical_filepath.to_str().unwrap()).await;
    if api_response_result.is_err() {
        eprintln!("Failed to call API: {}", api_response_result.err().unwrap());
        std::process::exit(1);
    }

    let api_response = api_response_result.unwrap();
    if !api_response.status().is_success() {
        eprintln!("Server returned error: {};{}", api_response.status(), api_response.text().await.unwrap());
        std::process::exit(1);
    }

    let deserialization_result = api_response.json::<Media>().await;
    if deserialization_result.is_err() {
        eprintln!("Deserialization error: {}", deserialization_result.err().unwrap());
        std::process::exit(1);
    }

    let media = deserialization_result.unwrap();
    println!("OK! Server response: {:?}", media);
}