extern crate core;

mod tags;
mod media;
mod hash;

use axum::{routing::*, Router};
use sqlx::postgres::{PgPoolOptions};
use std::{time::Duration};
use axum::extract::DefaultBodyLimit;
use sqlx::{Pool, Postgres};
use tracing::info;
use crate::media::{create_media, delete_media, get_all_media, get_media_by_id};
use crate::tags::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // configuration
    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost".to_string());
    let s3_bucket_name = std::env::var("S3_BUCKET_NAME")
        .unwrap_or_else(|_| "media".to_string());
    let s3_account_id = std::env::var("S3_ACCOUNT_ID")
        .unwrap_or_else(|_| "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string());
    let s3_access_key = std::env::var("S3_ACCESS_KEY")
        .unwrap_or_else(|_| "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string());
    let s3_secret_key = std::env::var("S3_SECRET_KEY")
        .unwrap_or_else(|_| "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string());
    let s3_public_url_prefix = std::env::var("S3_PUBLIC_URL")
        .unwrap_or_else(|_| "https://pub-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.r2.dev/".to_string());
    let s3_configuration = S3Configuration {
        bucket_name: s3_bucket_name,
        account_id: s3_account_id,
        access_key: s3_access_key,
        secret_key: s3_secret_key,
        public_url_prefix: s3_public_url_prefix,
    };
    dbg!(&db_connection_str);
    dbg!(&s3_configuration);

    // setup connection pool
    info!("Connecting pool to DB...");
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");
    info!("Connected to DB!");

    // build our application with some routes
    let app_state = AppState {
        pool: pg_pool,
        s3: s3_configuration,
    };
    let app = Router::new()
        .route("/ping", get(ping))
        .route("/ping_db", get(ping_db))
        .route("/tags", post(create_tag))
        .route("/tags", get(get_all_tags))
        .route("/tags/:tag_id", get(get_tag_by_id))
        .route("/tags/:tag_id", delete(delete_tag))
        .route("/media", post(create_media))
        .route("/media", get(get_all_media))
        .route("/media/:media_id", get(get_media_by_id))
        .route("/media/:media_id", delete(delete_media))
        .layer(DefaultBodyLimit::max(52_428_800))
        .with_state(app_state);

    // run it with hyper
    let addr = "[::]:3000".parse().unwrap();
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
    s3: S3Configuration,
}

#[derive(Debug, Clone)]
pub struct S3Configuration {
    bucket_name: String,
    account_id: String,
    access_key: String,
    secret_key: String,
    public_url_prefix: String,
}
