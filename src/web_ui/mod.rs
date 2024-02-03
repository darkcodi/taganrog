use std::iter::once;
use axum::{routing::get, Router, Json};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum_macros::FromRef;
use axum_template::engine::Engine;
use axum_template::{Key, RenderHtml};
use minijinja::Environment;
use tracing::{info, Level};
use tracing_subscriber::util::SubscriberInitExt;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use crate::api::client::ApiClient;
use crate::db::{Media, TagsAutocomplete};
use crate::utils::normalize_query;
use crate::utils::str_utils::StringExtensions;

const INDEX_TEMPLATE: &str = include_str!("templates/index.html");
const MEDIA_TEMPLATE: &str = include_str!("templates/media.html");
const SEARCH_TEMPLATE: &str = include_str!("templates/search.html");
const SEARCH_MORE_TEMPLATE: &str = include_str!("templates/search_more.html");
const TAGS_AUTOCOMPLETE_TEMPLATE: &str = include_str!("templates/tag_autocomplete.html");

pub async fn serve(api_url: &str) {
    let tracing_layer = tracing_subscriber::fmt::layer();
    let filter = filter::Targets::new()
        // .with_target("tower_http::trace::on_request", Level::DEBUG)
        .with_target("tower_http::trace::on_response", Level::DEBUG)
        .with_target("tower_http::trace::make_span", Level::DEBUG)
        .with_default(Level::INFO);
    tracing_subscriber::registry()
        .with(tracing_layer)
        .with(filter)
        .init();

    info!("initializing templates...");
    let mut jinja = Environment::new();
    jinja.add_template("/", INDEX_TEMPLATE).unwrap();
    jinja.add_template("/media/:media_id", MEDIA_TEMPLATE).unwrap();
    jinja.add_template("/search", SEARCH_TEMPLATE).unwrap();
    jinja.add_template("/search/more", SEARCH_MORE_TEMPLATE).unwrap();
    jinja.add_template("/tags/autocomplete", TAGS_AUTOCOMPLETE_TEMPLATE).unwrap();

    info!("initializing router...");
    let router = Router::new()
        .route("/", get(index))
        .route("/media/:media_id", get(media))
        .route("/media/:media_id/stream", get(stream_media))
        .route("/search", get(media_search))
        .route("/search/more", get(media_search_more))
        .route("/tags/autocomplete", get(autocomplete_tags))
        .with_state(AppState {
            engine: Engine::from(jinja),
            api_client: ApiClient::new(api_url.to_string()),
        })
        .layer(TraceLayer::new_for_http());

    let addr = "[::]:1775";
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind to address");
    info!("listening on {}", &addr);
    axum::serve(listener, router).await.expect("error running HTTP server");
}

type AppEngine = Engine<Environment<'static>>;

#[derive(Clone, FromRef)]
struct AppState {
    engine: AppEngine,
    api_client: ApiClient,
}

#[derive(Debug, Serialize)]
pub struct IndexPageContext;

async fn index(
    State(engine): State<AppEngine>,
    Key(key): Key,
) -> impl IntoResponse {
    let ctx = IndexPageContext;
    RenderHtml(key, engine, ctx)
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    p: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SearchPageContext {
    query: String,
    query_tags: Vec<String>,
    media_vec: Vec<Media>,
    next_page: u64,
    has_next: bool,
}

async fn media_search(
    State(engine): State<AppEngine>,
    Key(key): Key,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let query_tags = extract_tags(&query);
    RenderHtml(key, engine, SearchPageContext { query: query.q, query_tags, media_vec: vec![], next_page: 0, has_next: true })
}

async fn media_search_more(
    State(engine): State<AppEngine>,
    State(api_client): State<ApiClient>,
    Key(key): Key,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    if query.q.is_empty() {
        return RenderHtml(key, engine, SearchPageContext { query: "".to_string(), query_tags: vec![], media_vec: vec![], next_page: 0, has_next: false });
    }
    let page_index = query.p.unwrap_or(0);
    let page_size = 5;
    let api_response = api_client.search_media(&query.q, page_size, page_index).await.unwrap();
    let mut media_vec: Vec<Media> = api_response.json().await.unwrap();
    let has_next = media_vec.len() as u64 == page_size;

    // order tags in each media by the order they appear in the query
    let query_tags = extract_tags(&query);
    let tag_to_index = query_tags.clone().into_iter()
        .enumerate().map(|(i, x)| (x, i))
        .collect::<std::collections::HashMap<String, usize>>();
    media_vec.iter_mut().for_each(|media| {
        media.tags.sort_by_key(|x| tag_to_index.get(x).unwrap_or(&usize::MAX));
    });

    let ctx = SearchPageContext {
        query: query.q,
        query_tags,
        media_vec,
        next_page: page_index + 1,
        has_next,
    };
    RenderHtml(key, engine, ctx)
}

fn extract_tags(query: &SearchQuery) -> Vec<String> {
    let query_tags = query.q.split(" ")
        .map(|x| x.slugify().to_string())
        .filter(|x| !x.is_empty())
        .collect::<Vec<String>>();
    query_tags
}

#[derive(Debug, Serialize)]
struct AutocompleteObject {
    query: String,
    suggestion: String,
    highlighted_suggestion: String,
}

async fn autocomplete_tags(
    State(api_client): State<ApiClient>,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<AutocompleteObject>> {
    let normalized_query = normalize_query(&query.q);
    if normalized_query.is_empty() {
        return Json(vec![]);
    }
    let page = query.p.unwrap_or(0);
    let api_response = api_client.autocomplete_tags(&normalized_query, page).await.unwrap();
    let autocomplete: Vec<TagsAutocomplete> = api_response.json().await.unwrap();
    let autocomplete = autocomplete.iter().map(|x| {
        let query = normalized_query.clone();
        let suggestion = x.head.iter().map(|x| x.as_str())
            .chain(once(x.last.as_str()))
            .collect::<Vec<&str>>().join(" ");
        let highlighted_suggestion = query.clone() + "<mark>" + &suggestion[normalized_query.len()..] + "</mark>";
        AutocompleteObject { query, suggestion, highlighted_suggestion }
    }).collect::<Vec<AutocompleteObject>>();
    Json(autocomplete)
}

#[derive(Debug, Serialize)]
pub struct MediaPageContext {
    media: Media,
}

async fn media(
    State(engine): State<AppEngine>,
    State(api_client): State<ApiClient>,
    Path(media_id): Path<String>,
    Key(key): Key,
) -> impl IntoResponse {
    let api_response = api_client.get_media(&media_id).await.unwrap();
    let media: Media = api_response.json().await.unwrap();
    RenderHtml(key, engine, MediaPageContext { media })
}

async fn stream_media(
    State(api_client): State<ApiClient>,
    Path(media_id): Path<String>,
) -> impl IntoResponse {
    let api_response = api_client.stream_media(&media_id).await;
    match api_response {
        Ok(response) => {
            if response.status().is_success() {
                let bytes_stream = response.bytes_stream();
                let response = axum::http::Response::new(axum::body::Body::from_stream(bytes_stream));
                Ok(response)
            } else {
                Err(axum::http::StatusCode::from_u16(response.status().as_u16()).unwrap())
            }
        }
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}
