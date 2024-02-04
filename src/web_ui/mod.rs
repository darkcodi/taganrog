use std::iter::once;
use askama::Template;
use axum::{routing::get, Router, Json};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::delete;
use axum_macros::FromRef;
use chrono::{DateTime, Utc};
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

// icons
const FAVICON: &[u8] = include_bytes!("assets/favicon.ico");
const ICON_DEFAULT: &[u8] = include_bytes!("assets/icons/file.svg");
const ICON_JPG: &[u8] = include_bytes!("assets/icons/jpg.svg");
const ICON_MP3: &[u8] = include_bytes!("assets/icons/mp3.svg");
const ICON_MP4: &[u8] = include_bytes!("assets/icons/mp4.svg");
const ICON_PNG: &[u8] = include_bytes!("assets/icons/png.svg");

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

    info!("initializing router...");
    let router = Router::new()

        // icons
        .route("/favicon.ico", get(favicon))
        .route("/icons/file.svg", get(icon_default))
        .route("/icons/jpg.svg", get(icon_jpg))
        .route("/icons/mp3.svg", get(icon_mp3))
        .route("/icons/mp4.svg", get(icon_mp4))
        .route("/icons/png.svg", get(icon_png))

        // pages
        .route("/", get(index))
        .route("/media/:media_id", get(get_media).delete(delete_media))
        .route("/media/:media_id/add-tag", get(add_tag_to_media))
        .route("/media/:media_id/remove-tag", delete(delete_tag_from_media))
        .route("/search", get(media_search))
        .route("/search/more", get(media_search_more))

        // api
        .route("/media/:media_id/stream", get(stream_media))
        .route("/tags/search", get(search_tags))
        .route("/tags/autocomplete", get(autocomplete_tags))

        .with_state(AppState {
            api_client: ApiClient::new(api_url.to_string()),
        })
        .layer(TraceLayer::new_for_http());

    let addr = "[::]:1775";
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind to address");
    info!("listening on {}", &addr);
    axum::serve(listener, router).await.expect("error running HTTP server");
}

#[derive(Clone, FromRef)]
struct AppState {
    api_client: ApiClient,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
    where
        T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            ).into_response(),
        }
    }
}

#[derive(Default, Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    query: String,
}

async fn index() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate::default())
}

async fn favicon() -> impl IntoResponse { Response::<Body>::new(FAVICON.into()) }
async fn icon_default() -> impl IntoResponse { icon_response(ICON_DEFAULT) }
async fn icon_jpg() -> impl IntoResponse { icon_response(ICON_JPG) }
async fn icon_mp3() -> impl IntoResponse { icon_response(ICON_MP3) }
async fn icon_mp4() -> impl IntoResponse { icon_response(ICON_MP4) }
async fn icon_png() -> impl IntoResponse { icon_response(ICON_PNG) }

fn icon_response(icon: &'static [u8]) -> Response<Body> {
    let mut response = Response::<Body>::new(icon.into());
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "image/svg+xml".parse().unwrap());
    response
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    p: Option<u64>,
}

#[derive(Default, Template)]
#[template(path = "search.html")]
pub struct SearchTemplate {
    query: String,
}

#[derive(Default, Template)]
#[template(path = "search_more.html")]
pub struct SearchMoreTemplate {
    query: String,
    query_tags: Vec<String>,
    media_vec: Vec<ExtendedMedia>,
    next_page: u64,
    has_next: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct ExtendedMedia {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub size: i64,
    pub location: String,
    pub was_uploaded: bool,
    pub tags: Vec<ExtendedTag>,
}

impl From<Media> for ExtendedMedia {
    fn from(media: Media) -> Self {
        let tags = media.tags.into_iter().map(|tag| {
            ExtendedTag {
                name: tag,
                is_in_query: false,
            }
        }).collect();
        Self {
            id: media.id,
            filename: media.filename,
            content_type: media.content_type,
            created_at: media.created_at,
            size: media.size,
            location: media.location,
            was_uploaded: media.was_uploaded,
            tags,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ExtendedTag {
    pub name: String,
    pub is_in_query: bool,
}

async fn media_search(Query(query): Query<SearchQuery>) -> impl IntoResponse {
    HtmlTemplate(SearchTemplate { query: normalize_query(&query.q.unwrap_or_default()) })
}

async fn media_search_more(
    State(api_client): State<ApiClient>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let normalized_query = normalize_query(&query.q.unwrap_or_default());
    if normalized_query.is_empty() {
        return HtmlTemplate(SearchMoreTemplate::default());
    }
    let page_index = query.p.unwrap_or(0);
    let page_size = 10;
    let api_response = api_client.search_media(&normalized_query, page_size, page_index).await.unwrap();
    let media_vec: Vec<Media> = api_response.json().await.unwrap();
    let mut media_vec = media_vec.into_iter().map(|x| x.into()).collect::<Vec<ExtendedMedia>>();
    let has_next = media_vec.len() as u64 == page_size;

    // order tags in each media by the order they appear in the query
    let query_tags = extract_tags(&normalized_query);
    let tag_to_index = query_tags.clone().into_iter()
        .enumerate().map(|(i, x)| (x, i))
        .collect::<std::collections::HashMap<String, usize>>();
    media_vec.iter_mut().for_each(|media| {
        media.tags.sort_by_key(|x| tag_to_index.get(&x.name).unwrap_or(&usize::MAX));
        media.tags.iter_mut().for_each(|tag| {
            tag.is_in_query = query_tags.contains(&tag.name);
        });
    });

    HtmlTemplate(SearchMoreTemplate {
        query: normalized_query,
        query_tags,
        media_vec,
        next_page: page_index + 1,
        has_next,
    })
}

fn extract_tags(query: &String) -> Vec<String> {
    let query_tags = query.split(" ")
        .map(|x| x.slugify().to_string())
        .filter(|x| !x.is_empty())
        .collect::<Vec<String>>();
    query_tags
}

#[derive(Debug, Default, Deserialize)]
pub struct TagBody {
    q: Option<String>,
    tag: Option<String>,
}

#[derive(Default, Template)]
#[template(path = "add_tag_to_media.html")]
pub struct AddTagToMediaTemplate {
    media: Media,
    tag: String,
    query: String,
}

async fn add_tag_to_media(
    State(api_client): State<ApiClient>,
    Path(media_id): Path<String>,
    Query(query): Query<TagBody>,
) -> impl IntoResponse {
    let tag = query.tag.unwrap_or_default();
    if tag.is_empty() {
        return HtmlTemplate(AddTagToMediaTemplate::default());
    }
    let normalized_query = normalize_query(&query.q.unwrap_or_default());
    let api_response = api_client.add_tag_to_media(&media_id, &tag).await.unwrap();
    let media: Media = api_response.json().await.unwrap();
    HtmlTemplate(AddTagToMediaTemplate { media, tag, query: normalized_query })
}

async fn delete_tag_from_media(
    State(api_client): State<ApiClient>,
    Path(media_id): Path<String>,
    Query(query): Query<TagBody>,
) -> impl IntoResponse {
    let tag = query.tag.unwrap_or_default();
    if tag.is_empty() {
        return Response::new(Body::empty());
    }
    let api_response = api_client.delete_tag_from_media(&media_id, &tag).await.unwrap();
    let _: Media = api_response.json().await.unwrap();
    Response::new(Body::empty())
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
    let normalized_query = normalize_query(&query.q.unwrap_or_default());
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

async fn search_tags(
    State(api_client): State<ApiClient>,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<AutocompleteObject>> {
    let mut normalized_query = normalize_query(&query.q.unwrap_or_default());
    if normalized_query.is_empty() || normalized_query.contains(" ") {
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
        let highlighted_suggestion = match suggestion.starts_with(&query) {
            true => query.clone() + "<mark>" + &suggestion[normalized_query.len()..] + "</mark>",
            false => suggestion.clone(),
        };
        AutocompleteObject { query, suggestion, highlighted_suggestion }
    }).collect::<Vec<AutocompleteObject>>();
    Json(autocomplete)
}

#[derive(Default, Template)]
#[template(path = "media.html")]
pub struct MediaPageTemplate {
    query: String,
    media: Media,
}

async fn get_media(
    State(api_client): State<ApiClient>,
    Query(query): Query<SearchQuery>,
    Path(media_id): Path<String>,
) -> impl IntoResponse {
    let query = normalize_query(&query.q.unwrap_or_default());
    let api_response = api_client.get_media(&media_id).await.unwrap();
    let media: Media = api_response.json().await.unwrap();
    HtmlTemplate(MediaPageTemplate { query, media })
}

async fn delete_media(
    State(api_client): State<ApiClient>,
    Path(media_id): Path<String>,
) -> impl IntoResponse {
    let api_response = api_client.delete_media(&media_id).await;
    if api_response.is_err() || !api_response.unwrap().status().is_success() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
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
                let response = Response::new(axum::body::Body::from_stream(bytes_stream));
                Ok(response)
            } else {
                Err(StatusCode::from_u16(response.status().as_u16()).unwrap())
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
