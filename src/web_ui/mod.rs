mod commands;
mod streaming;

use std::hash::Hasher;
use std::sync::Arc;
use askama::Template;
use axum::{routing::get, Router};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum_macros::FromRef;
use chrono::{DateTime, Utc};
use http::{header::*, response::Builder as ResponseBuilder};
use itertools::Itertools;
use log::info;
use random_port::{PortPicker, Protocol};
use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use crate::client::TaganrogClient;
use crate::config::AppConfig;
use crate::entities::{Media, TagsAutocomplete};
use crate::storage::FileStorage;
use crate::utils::normalize_query;
use crate::utils::str_utils::StringExtensions;
use crate::web_ui::commands::*;
use crate::web_ui::streaming::get_stream_response;

// icons
const FAVICON: &[u8] = include_bytes!("assets/favicon.ico");
const DEFAULT_THUMBNAIL: &[u8] = include_bytes!("assets/icons/default_thumbnail.svg");

// scripts
const ALGOLIA_LIB: &[u8] = include_bytes!("assets/scripts/algolia_1.15.1.min.js");
const AWESOME_CLOUD_LIB: &[u8] = include_bytes!("assets/scripts/awesome_cloud_0.2.min.js");
const JQUERY_LIB: &[u8] = include_bytes!("assets/scripts/jquery_2.1.0.min.js");
const TAILWIND_LIB: &[u8] = include_bytes!("assets/scripts/tailwind_1.0.8.min.js");
const TAILWIND_EXT_LIB: &[u8] = include_bytes!("assets/scripts/tailwind_ext_1.0.8.min.js");

// styles
const ALGOLIA_STYLES: &[u8] = include_bytes!("assets/styles/algolia_classic_1.15.1.min.css");

const DEFAULT_MEDIA_PAGE_SIZE: usize = 6;
const DEFAULT_AUTOCOMPLETE_PAGE_SIZE: usize = 6;

pub async fn serve(config: AppConfig, client: TaganrogClient<FileStorage>) {
    let media_count = client.get_media_count();
    info!("media count: {}", media_count);

    info!("initializing router...");
    let app_state = AppState { config: Arc::new(config), client: Arc::new(RwLock::new(client)) };
    let router = Router::new()
        // icons
        .route("/favicon.ico", get(favicon))
        .route("/default_thumbnail.svg", get(get_default_thumbnail))

        // scripts
        .route("/scripts/algolia.min.js", get(get_algolia_lib))
        .route("/scripts/awesome_cloud.min.js", get(get_awesome_cloud_lib))
        .route("/scripts/jquery.min.js", get(get_jquery_lib))
        .route("/scripts/tailwind.min.js", get(get_tailwind_lib))
        .route("/scripts/tailwind_ext.min.js", get(get_tailwind_ext_lib))

        // styles
        .route("/styles/algolia.min.css", get(get_algolia_styles))

        // pages
        .route("/", get(index))
        .route("/media/add", get(add_media_page))
        .route("/media/random", get(get_random_media))
        .route("/media/:media_id", get(get_media))
        .route("/search", get(media_search))
        .route("/tags_cloud", get(tags_cloud))

        .with_state(app_state.clone())
        .layer(TraceLayer::new_for_http());

    let port: u16 = PortPicker::new().protocol(Protocol::Tcp).pick().unwrap();
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("failed to bind to address");
    info!("listening on {}", &addr);

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("error running HTTP server");
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![choose_files, load_media_from_file, has_thumbnail, save_thumbnail, add_tag_to_media, remove_tag_from_media, delete_media, autocomplete_tags])
        .setup(move |app| {
            app.manage(app_state);
            let url = format!("http://localhost:{}", port).parse().unwrap();
            WebviewWindowBuilder::new(app, "main".to_string(), WebviewUrl::External(url))
                .title("Taganrog")
                .inner_size(1024.0, 768.0)
                .resizable(true)
                .fullscreen(false)
                .build()?;
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("stream", move |_ctx, request, responder| {
            match get_stream_response(request) {
                Ok(http_response) => responder.respond(http_response),
                Err(e) => responder.respond(
                    ResponseBuilder::new()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(CONTENT_TYPE, "text/plain")
                        .body(e.to_string().as_bytes().to_vec())
                        .unwrap(),
                ),
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[derive(Clone, FromRef)]
struct AppState {
    config: Arc<AppConfig>,
    client: Arc<RwLock<TaganrogClient<FileStorage>>>,
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

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    p: Option<usize>,
    ps: Option<usize>,
    path: Option<String>,
}

#[derive(Default, Template)]
#[template(path = "search.html")]
pub struct SearchTemplate {
    query: String,
    media_vec: Vec<ExtendedMedia>,
    current_page_number: usize,
    max_page_number: usize,
    page_navigation: Vec<usize>,
    min_page_navigation: usize,
    max_page_navigation: usize,
    has_pages_before: bool,
    has_pages_after: bool,
    time_elapsed_ms: u128,
}

impl SearchTemplate {
    pub fn is_current_page(&self, page: &&usize) -> bool {
        self.current_page_number == **page
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ExtendedMedia {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub size: i64,
    pub location: String,
    pub location_url: String,
    pub thumbnail_location: String,
    pub thumbnail_location_url: String,
    pub tags: Vec<ExtendedTag>,
    pub is_image: bool,
}

impl ExtendedMedia {
    pub fn create_for_query(media: Media, app_config: &AppConfig, query_tags: &Vec<String>) -> Self {
        let mut media = ExtendedMedia::create(media, app_config);
        media.tags.sort_by_key(|ex_tag| query_tags.iter().position(|tag| tag == &ex_tag.name).unwrap_or(usize::MAX));
        media.tags.iter_mut().for_each(|tag| {
            tag.is_in_query = query_tags.contains(&tag.name);
        });
        media
    }

    pub fn create(media: Media, app_config: &AppConfig) -> Self {
        let tags = media.tags.into_iter().map(|tag| {
            let mut tag: ExtendedTag = tag.into();
            tag.is_in_query = false;
            tag
        }).collect();
        let location_url = convert_file_src(&media.location);
        let thumbnail_location = app_config.thumbnails_dir.join(format!("{}.png", &media.id)).to_string_lossy().to_string();
        let thumbnail_location_url = if std::path::Path::new(&thumbnail_location).exists() {
            convert_file_src(&thumbnail_location)
        } else {
            "/default_thumbnail.svg".to_string()
        };
        Self {
            id: media.id,
            filename: media.filename,
            created_at: media.created_at,
            size: media.size,
            location: media.location,
            location_url,
            thumbnail_location,
            thumbnail_location_url,
            tags,
            is_image: media.content_type.starts_with("image"),
            content_type: media.content_type,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ExtendedTag {
    pub name: String,
    pub is_in_query: bool,
    pub bg_color: String,
    pub fg_color: String,
}

impl From<String> for ExtendedTag {
    fn from(tag: String) -> Self {
        let bg_color = get_bg_color(&tag);
        let fg_color = get_fg_color(&bg_color);
        Self {
            name: tag,
            is_in_query: false,
            bg_color,
            fg_color,
        }
    }
}

async fn media_search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let normalized_query = normalize_query(&query.q.unwrap_or_default());
    if normalized_query.is_empty() {
        return HtmlTemplate(SearchTemplate::default());
    }
    let page_number = query.p.unwrap_or(1).max(1);
    let page_index = page_number - 1;
    let page_size = query.ps.unwrap_or(DEFAULT_MEDIA_PAGE_SIZE).max(1);

    let client = state.client.read().await;
    let media_page = match normalized_query.as_str() {
        "all" => client.get_all_media(page_size, page_index),
        "null" => client.get_untagged_media(page_size, page_index),
        "untagged" => client.get_untagged_media(page_size, page_index),
        "no-tags" => client.get_untagged_media(page_size, page_index),
        "no-thumbnail" => client.get_media_without_thumbnail(page_size, page_index),
        _ => client.search_media(&normalized_query, page_size, page_index),
    };
    drop(client);

    let page_number = media_page.page_index + 1;

    let query_tags = extract_tags(&normalized_query);
    let media_vec = media_page.media_vec.into_iter()
        .map(|x| ExtendedMedia::create_for_query(x, &state.config, &query_tags))
        .collect::<Vec<ExtendedMedia>>();

    const PAGES_BEFORE: usize = 3;
    const PAGES_AFTER: usize = 3;
    let pages_navigation = (page_number.saturating_sub(PAGES_BEFORE)..=page_number.saturating_add(PAGES_AFTER))
        .filter(|x| *x > 0 && *x <= media_page.total_pages).collect::<Vec<usize>>();
    let min_page_navigation = pages_navigation.first().cloned().unwrap_or(0);
    let max_page_navigation = pages_navigation.last().cloned().unwrap_or(0);
    let has_more_pages_before = min_page_navigation > 2;
    let has_more_pages_after = max_page_navigation + 1 < media_page.total_pages;
    let time_elapsed_ms = start.elapsed().as_millis();

    HtmlTemplate(SearchTemplate {
        query: normalized_query,
        media_vec,
        current_page_number: page_number,
        max_page_number: media_page.total_pages,
        page_navigation: pages_navigation,
        min_page_navigation,
        max_page_navigation,
        has_pages_before: has_more_pages_before,
        has_pages_after: has_more_pages_after,
        time_elapsed_ms,
    })
}

fn extract_tags(query: &str) -> Vec<String> {
    let query_tags = query.split(' ')
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
    media_count: usize,
}

#[derive(Default, Template)]
#[template(path = "media.html")]
pub struct MediaPageTemplate {
    query: String,
    page: usize,
    media: ExtendedMedia,
    media_exists: bool,
}

async fn get_media(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
    Path(media_id): Path<String>,
) -> impl IntoResponse {
    let normalized_query = normalize_query(&query.q.unwrap_or_default());
    let page = query.p.unwrap_or(1);
    let client = state.client.read().await;
    let mut maybe_media = client.get_media_by_id(&media_id);
    if maybe_media.is_none() {
        maybe_media = client.create_media_from_file(&query.path.unwrap_or_default().into()).await.ok();
    }
    if let Some(media) = maybe_media {
        let media = ExtendedMedia::create(media, &state.config);
        HtmlTemplate(MediaPageTemplate { query: normalized_query, page, media, media_exists: true })
    } else {
        HtmlTemplate(MediaPageTemplate { query: normalized_query, page, media: ExtendedMedia::default(), media_exists: false })
    }
}

async fn get_random_media(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let client = state.client.read().await;
    match client.get_random_media() {
        Some(media) => {
            let media = ExtendedMedia::create(media, &state.config);
            HtmlTemplate(MediaPageTemplate { query: "".to_string(), page: 1, media, media_exists: true })
        },
        None => HtmlTemplate(MediaPageTemplate { query: "".to_string(), page: 1, media: ExtendedMedia::default(), media_exists: false })
    }
}

async fn get_default_thumbnail() -> impl IntoResponse {
    let mut response = Response::new(Body::from(DEFAULT_THUMBNAIL));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "image/svg+xml".parse().unwrap());
    response
}

async fn get_algolia_lib() -> impl IntoResponse {
    let mut response = Response::new(Body::from(ALGOLIA_LIB));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "application/javascript".parse().unwrap());
    response
}

async fn get_awesome_cloud_lib() -> impl IntoResponse {
    let mut response = Response::new(Body::from(AWESOME_CLOUD_LIB));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "application/javascript".parse().unwrap());
    response
}

async fn get_jquery_lib() -> impl IntoResponse {
    let mut response = Response::new(Body::from(JQUERY_LIB));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "application/javascript".parse().unwrap());
    response
}

async fn get_tailwind_lib() -> impl IntoResponse {
    let mut response = Response::new(Body::from(TAILWIND_LIB));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "application/javascript".parse().unwrap());
    response
}

async fn get_tailwind_ext_lib() -> impl IntoResponse {
    let mut response = Response::new(Body::from(TAILWIND_EXT_LIB));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "application/javascript".parse().unwrap());
    response
}

async fn get_algolia_styles() -> impl IntoResponse {
    let mut response = Response::new(Body::from(ALGOLIA_STYLES));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "text/css".parse().unwrap());
    response
}

#[derive(Default, Template)]
#[template(path = "add_media.html")]
struct AddMediaTemplate { }

async fn add_media_page() -> impl IntoResponse {
    HtmlTemplate(AddMediaTemplate::default())
}

#[derive(Default, Debug, Serialize)]
pub struct MultipartFile {
    pub file_name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub preview_file_name: String,
    pub preview_content_type: String,
    pub preview_bytes: Vec<u8>,
}

fn get_bg_color(text: &str) -> String {
    const MAX_VALUE: u64 = 0xFFFFFF;
    let mut hasher = twox_hash::XxHash64::default();
    hasher.write(text.as_bytes());
    let hash = hasher.finish();
    let color = hash % MAX_VALUE;
    let color_str = format!("#{:06x}", color);
    color_str
}

fn get_fg_color(bg_color: &str) -> String {
    let bg_color = bg_color.trim_start_matches('#');
    let r = u8::from_str_radix(&bg_color[0..2], 16).unwrap();
    let g = u8::from_str_radix(&bg_color[2..4], 16).unwrap();
    let b = u8::from_str_radix(&bg_color[4..6], 16).unwrap();
    let yiq = ((r as f32 * 299.0) + (g as f32 * 587.0) + (b as f32 * 114.0)) / 1000.0;
    let fg_color = if yiq >= 128.0 { "black" } else { "white" };
    fg_color.to_string()
}

#[derive(Default, Template)]
#[template(path = "tags_cloud.html")]
pub struct TagsCloudTemplate {
    query: String,
    tags: Vec<TagsAutocomplete>,
}

async fn tags_cloud(
    Query(query): Query<SearchQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let normalized_query = normalize_query(&query.q.unwrap_or_default());
    let client = state.client.read().await;
    let tags = client.get_all_tags();
    drop(client);
    let tags = tags.iter()
        .sorted_by_key(|x| x.media_count).rev()
        .take(100)
        .cloned()
        .collect::<Vec<TagsAutocomplete>>();
    HtmlTemplate(TagsCloudTemplate { query: normalized_query, tags })
}

pub fn convert_file_src(file_path: &str) -> String {
    let encoded_path = urlencoding::encode(file_path);
    format!("http://stream.localhost/{}", encoded_path)
}
