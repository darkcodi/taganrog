use axum::routing::get;
use axum::Router;

pub fn router() -> Router {
    Router::new()
        .route("/api/ping", get(ping))
}

async fn ping() -> String {
    "pong".to_string()
}
