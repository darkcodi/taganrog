use axum::{routing::get, Router};
use axum::response::IntoResponse;
use axum_macros::FromRef;
use axum_template::engine::Engine;
use axum_template::{Key, RenderHtml};
use minijinja::Environment;
use tracing::info;
use tracing_subscriber::util::SubscriberInitExt;
use serde::Serialize;

const INDEX_TEMPLATE: &str = include_str!("../templates/index.html");

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("initializing router...");
    let mut jinja = Environment::new();
    jinja
        .add_template("/", INDEX_TEMPLATE)
        .unwrap();

    let router = Router::new()
        .route("/", get(index))
        .with_state(AppState {
            engine: Engine::from(jinja),
        });;

    let addr = "[::]:1775";
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind to address");
    info!("listening on {}", &addr);
    axum::serve(listener, router).await.expect("error running HTTP server");
}

type AppEngine = Engine<Environment<'static>>;

#[derive(Clone, FromRef)]
struct AppState {
    engine: AppEngine,
}

#[derive(Debug, Serialize)]
pub struct IndexPageContext;

async fn index(
    engine: AppEngine,
    Key(key): Key,
) -> impl IntoResponse {
    let ctx = IndexPageContext;
    RenderHtml(key, engine, ctx)
}
