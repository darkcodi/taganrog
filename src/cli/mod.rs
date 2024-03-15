use std::path::PathBuf;
use tracing::error;
use crate::client::TaganrogClient;
use crate::config;
use crate::config::AppConfig;
use crate::entities::InsertResult;

pub async fn add_media(config: AppConfig, filepath: &str) {
    let canonical_filepath_result = std::fs::canonicalize(filepath);
    if canonical_filepath_result.is_err() {
        let err = canonical_filepath_result.err().unwrap();
        if err.kind() == std::io::ErrorKind::NotFound {
            error!("File not found: {}", filepath);
            std::process::exit(1);
        } else {
            error!("IO Error: {}", err);
            std::process::exit(1);
        }
    }

    let canonical_filepath = canonical_filepath_result.unwrap();
    println!("File: {:?}", canonical_filepath);

    let mut client = TaganrogClient::new(config);
    let init_result = client.init().await;
    if init_result.is_err() {
        error!("Failed to initialize client: {}", init_result.err().unwrap());
        std::process::exit(1);
    }

    let create_result = client.create_media_from_file(canonical_filepath).await;
    if create_result.is_err() {
        error!("Failed to create media: {}", create_result.err().unwrap());
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

pub fn get_config_value(config: AppConfig, key: &str) {
    match key {
        "work-dir" => {
            println!("Workdir: {:?}", config.file_config.workdir);
            std::process::exit(0);
        },
        "upload-dir" => {
            println!("Upload dir: {:?}", config.file_config.upload_dir);
            std::process::exit(0);
        },
        _ => {
            eprintln!("Invalid key: {}", key);
            std::process::exit(1);
        }
    }
}

pub fn set_config_value(mut config: AppConfig, key: &str, value: &str) {
    match key {
        "work-dir" => {
            let path_result = PathBuf::try_from(value);
            if path_result.is_err() {
                eprintln!("Invalid path: {}", value);
                std::process::exit(1);
            }
            let path = path_result.unwrap();
            if !path.exists() {
                eprintln!("Path does not exist: {:?}", path);
                std::process::exit(1);
            }
            let path_str = path.display().to_string();
            config.file_config.workdir = Some(path_str);
            config::write_file_config(&config.config_path, &config.file_config);
            println!("Workdir set to: {:?}", value);
            std::process::exit(0);
        },
        "upload-dir" => {
            let path_result = PathBuf::try_from(value);
            if path_result.is_err() {
                eprintln!("Invalid path: {}", value);
                std::process::exit(1);
            }
            let path = path_result.unwrap();
            if !path.exists() {
                eprintln!("Path does not exist: {:?}", path);
                std::process::exit(1);
            }
            let path_str = path.display().to_string();
            config.file_config.upload_dir = Some(path_str);
            config::write_file_config(&config.config_path, &config.file_config);
            println!("Upload dir set to: {:?}", value);
            std::process::exit(0);
        },
        _ => {
            eprintln!("Invalid key: {}", key);
            std::process::exit(1);
        }
    }
}