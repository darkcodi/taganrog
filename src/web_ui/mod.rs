use axum::{routing::get, Router};
use axum::extract::{Query, State};
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
use crate::utils::str_utils::StringExtensions;

const INDEX_TEMPLATE: &str = include_str!("templates/index.html");
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
    jinja.add_template("/search", SEARCH_TEMPLATE).unwrap();
    jinja.add_template("/search/more", SEARCH_MORE_TEMPLATE).unwrap();
    jinja.add_template("/tags/autocomplete", TAGS_AUTOCOMPLETE_TEMPLATE).unwrap();

    info!("initializing router...");
    let router = Router::new()
        .route("/", get(index))
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
    media_vec: Vec<Media>,
    next_page: u64,
    has_next: bool,
}

async fn media_search(
    State(engine): State<AppEngine>,
    Key(key): Key,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    RenderHtml(key, engine, SearchPageContext { query: query.q, media_vec: vec![], next_page: 0, has_next: true })
}

async fn media_search_more(
    State(engine): State<AppEngine>,
    State(api_client): State<ApiClient>,
    Key(key): Key,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    if query.q.is_empty() {
        return RenderHtml(key, engine, SearchPageContext { query: "".to_string(), media_vec: vec![], next_page: 0, has_next: false });
    }
    let page_index = query.p.unwrap_or(0);
    let page_size = 5;
    let api_response = api_client.search_media(&query.q, page_size, page_index).await.unwrap();
    let mut media_vec: Vec<Media> = api_response.json().await.unwrap();
    let has_next = media_vec.len() as u64 == page_size;

    // order tags in each media by the order they appear in the query
    let tag_to_index = query.q.split(" ")
        .map(|x| x.slugify().to_string())
        .filter(|x| !x.is_empty())
        .enumerate().map(|(i, x)| (x, i))
        .collect::<std::collections::HashMap<String, usize>>();
    media_vec.iter_mut().for_each(|media| {
        media.tags.sort_by_key(|x| tag_to_index.get(x).unwrap_or(&usize::MAX));
    });

    let ctx = SearchPageContext {
        query: query.q,
        media_vec,
        next_page: page_index + 1,
        has_next,
    };
    RenderHtml(key, engine, ctx)
}

#[derive(Debug, Serialize)]
pub struct EnhancedTagsAutocomplete {
    last_tag: String,
    full_query: String,
}

#[derive(Debug, Serialize)]
pub struct TagSearchPageContext {
    suggestions: Vec<EnhancedTagsAutocomplete>,
}

async fn autocomplete_tags(
    State(engine): State<AppEngine>,
    State(api_client): State<ApiClient>,
    Key(key): Key,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    if query.q.is_empty() {
        return RenderHtml(key, engine, TagSearchPageContext { suggestions: vec![] });
    }
    let page = query.p.unwrap_or(0);
    let api_response = api_client.autocomplete_tags(&query.q, page).await.unwrap();
    let autocomplete: Vec<TagsAutocomplete> = api_response.json().await.unwrap();
    let suggestions: Vec<EnhancedTagsAutocomplete> = autocomplete.into_iter().map(|tag| {
        let head = tag.head;
        let last = tag.last;
        let full_query = format!("{} {}", head.join(" "), last);
        EnhancedTagsAutocomplete { last_tag: last, full_query }
    }).collect();
    let ctx = TagSearchPageContext { suggestions };
    RenderHtml(key, engine, ctx)
}
