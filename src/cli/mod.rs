use crate::client::{TaganrogClient, TaganrogConfig};
use crate::entities::InsertResult;

pub async fn add_media(config: TaganrogConfig, filepath: &str) {
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

    let mut client = TaganrogClient::new(config);
    let init_result = client.init().await;
    if init_result.is_err() {
        eprintln!("Failed to initialize client: {}", init_result.err().unwrap());
        std::process::exit(1);
    }

    let create_result = client.create_media_from_file(canonical_filepath).await;
    if create_result.is_err() {
        eprintln!("Failed to create media: {}", create_result.err().unwrap());
        std::process::exit(1);
    }
    let media = create_result.unwrap();
    match media {
        InsertResult::Existing(existing_media) => {
            println!("Media already exists: {:?}", existing_media);
            std::process::exit(0);
        }
        InsertResult::New(new_media) => {
            println!("Media created: {:?}", new_media);
            std::process::exit(0);
        }
    }
}