mod tags;

use axum::{routing::get, routing::post, Router};
use sqlx::postgres::{PgPoolOptions};
use std::{time::Duration};
use tracing::info;
use crate::tags::{create_tag, test_db};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost".to_string());

    // setup connection pool
    info!("Connecting pool to DB...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");
    info!("Connected to DB!");

    // build our application with some routes
    let app = Router::new()
        .route("/test_db", get(test_db))
        .route("/tags", post(create_tag))
        .with_state(pool);

    // run it with hyper
    let addr = "[::]:3000".parse().unwrap();
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
