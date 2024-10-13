use std::hash::Hasher;
use std::iter::once;
use std::sync::Arc;
use askama::Template;
use axum::{routing::get, Router, Json};
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, post};
use axum_macros::FromRef;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::info;
use random_port::{PortPicker, Protocol};
use serde::{Deserialize, Serialize};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use crate::client::TaganrogClient;
use crate::entities::{Media, TagsAutocomplete};
use crate::storage::FileStorage;
use crate::utils::normalize_query;
use crate::utils::str_utils::StringExtensions;

// icons
const FAVICON: &[u8] = include_bytes!("assets/favicon.ico");
const DEFAULT_THUMBNAIL: &[u8] = include_bytes!("assets/icons/default_thumbnail.svg");

// scripts
const ALGOLIA_LIB: &[u8] = include_bytes!("assets/scripts/algolia_1.15.1.min.js");
const AWESOME_CLOUD_LIB: &[u8] = include_bytes!("assets/scripts/awesome_cloud_0.2.min.js");
const HTMX_LIB: &[u8] = include_bytes!("assets/scripts/htmx_2.0.3.min.js");
const JQUERY_LIB: &[u8] = include_bytes!("assets/scripts/jquery_2.1.0.min.js");
const TAILWIND_LIB: &[u8] = include_bytes!("assets/scripts/tailwind_1.0.8.min.js");
const TAILWIND_EXT_LIB: &[u8] = include_bytes!("assets/scripts/tailwind_ext_1.0.8.min.js");

// styles
const ALGOLIA_STYLES: &[u8] = include_bytes!("assets/styles/algolia_classic_1.15.1.min.css");

const MAX_UPLOAD_SIZE_IN_BYTES: usize = 524_288_000; // 500 MB
const DEFAULT_MEDIA_PAGE_SIZE: usize = 6;
const DEFAULT_AUTOCOMPLETE_PAGE_SIZE: usize = 6;

pub async fn serve(client: TaganrogClient<FileStorage>) {
    let media_count = client.get_media_count();
    info!("media count: {}", media_count);

    info!("initializing router...");
    let router = Router::new()
        // icons
        .route("/favicon.ico", get(favicon))

        // scripts
        .route("/scripts/algolia.min.js", get(get_algolia_lib))
        .route("/scripts/awesome_cloud.min.js", get(get_awesome_cloud_lib))
        .route("/scripts/htmx.min.js", get(get_htmx_lib))
        .route("/scripts/jquery.min.js", get(get_jquery_lib))
        .route("/scripts/tailwind.min.js", get(get_tailwind_lib))
        .route("/scripts/tailwind_ext.min.js", get(get_tailwind_ext_lib))

        // styles
        .route("/styles/algolia.min.css", get(get_algolia_styles))

        // pages
        .route("/", get(index))
        .route("/media/random", get(get_random_media))
        .route("/media/:media_id", get(get_media).delete(delete_media))
        .route("/media/:media_id/add-tag", get(add_tag_to_media))
        .route("/media/:media_id/remove-tag", delete(remove_tag_from_media))
        .route("/search", get(media_search))
        .route("/upload", get(upload_page))
        .route("/tags_cloud", get(tags_cloud))

        // api
        .route("/media/:media_id/thumbnail", get(get_media_thumbnail))
        .route("/media/:media_id/stream", get(stream_media))
        .route("/tags/autocomplete", get(autocomplete_tags))
        .route("/upload/files", post(upload_files))

        .with_state(AppState {
            client: Arc::new(RwLock::new(client)),
        })
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_IN_BYTES));

    let port: u16 = PortPicker::new().protocol(Protocol::Tcp).pick().unwrap();
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("failed to bind to address");
    info!("listening on {}", &addr);

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("error running HTTP server");
    });

    tauri::Builder::default()
        // .invoke_handler(tauri::generate_handler![choose_file])
        .setup(move |app| {
            let url = format!("http://localhost:{}", port).parse().unwrap();
            WebviewWindowBuilder::new(app, "main".to_string(), WebviewUrl::External(url))
                .title("Taganrog")
                .inner_size(1024.0, 768.0)
                .resizable(true)
                .fullscreen(false)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[derive(Clone, FromRef)]
struct AppState {
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
    pub was_uploaded: bool,
    pub tags: Vec<ExtendedTag>,
    pub is_image: bool,
}

impl From<Media> for ExtendedMedia {
    fn from(media: Media) -> Self {
        let tags = media.tags.into_iter().map(|tag| {
            let mut tag: ExtendedTag = tag.into();
            tag.is_in_query = false;
            tag
        }).collect();
        Self {
            id: media.id,
            filename: media.filename,
            created_at: media.created_at,
            size: media.size,
            location: media.location,
            was_uploaded: media.was_uploaded,
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
    let mut media_vec = media_page.media_vec.into_iter().map(|x| x.into()).collect::<Vec<ExtendedMedia>>();

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

#[derive(Debug, Default, Deserialize)]
pub struct TagBody {
    tags: Option<String>,
}

#[derive(Default, Template)]
#[template(path = "add_tag_to_media.html")]
pub struct AddTagToMediaTemplate {
    media: Media,
    added_tags: Vec<ExtendedTag>,
}

async fn add_tag_to_media(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    Query(tag_body): Query<TagBody>,
) -> impl IntoResponse {
    let tags_str = normalize_query(&tag_body.tags.unwrap_or_default());
    let tags_str = tags_str.trim_end();
    if tags_str.is_empty() {
        return HtmlTemplate(AddTagToMediaTemplate::default());
    }
    let client = state.client.read().await;
    let maybe_media = client.get_media_by_id(&media_id);
    drop(client);

    if maybe_media.is_none() {
        return HtmlTemplate(AddTagToMediaTemplate::default());
    }
    let media = maybe_media.unwrap();
    let tags = extract_tags(tags_str);
    let new_tags = tags.iter().filter(|x| !media.tags.contains(x)).cloned().collect::<Vec<String>>();
    if new_tags.is_empty() {
        return HtmlTemplate(AddTagToMediaTemplate::default());
    }

    let mut client = state.client.write().await;
    for tag in &new_tags {
        client.add_tag_to_media(&media_id, tag).await.unwrap();
    }
    drop(client);

    let added_tags = new_tags.iter().map(|x| {
        let bg_color = get_bg_color(x);
        let fg_color = get_fg_color(&bg_color);
        ExtendedTag { name: x.clone(), is_in_query: false, bg_color, fg_color }
    }).collect::<Vec<ExtendedTag>>();
    HtmlTemplate(AddTagToMediaTemplate { media, added_tags })
}

async fn remove_tag_from_media(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    Query(tag_body): Query<TagBody>,
) -> impl IntoResponse {
    let tags_str = normalize_query(&tag_body.tags.unwrap_or_default());
    let tags_str = tags_str.trim_end();
    if tags_str.is_empty() {
        return Response::new(Body::empty());
    }
    let client = state.client.read().await;
    let maybe_media = client.get_media_by_id(&media_id);
    drop(client);

    if maybe_media.is_none() {
        return Response::new(Body::empty());
    }
    let media = maybe_media.unwrap();
    let tags = extract_tags(tags_str);
    let removed_tags = tags.iter().filter(|x| media.tags.contains(x)).cloned().collect::<Vec<String>>();
    if removed_tags.is_empty() {
        return Response::new(Body::empty());
    }

    let mut client = state.client.write().await;
    for tag in &removed_tags {
        client.remove_tag_from_media(&media_id, tag).await.unwrap();
    }
    Response::new(Body::empty())
}

#[derive(Debug, Serialize)]
struct AutocompleteObject {
    query: String,
    suggestion: String,
    highlighted_suggestion: String,
    media_count: usize,
}

async fn autocomplete_tags(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<AutocompleteObject>> {
    let normalized_query = normalize_query(&query.q.unwrap_or_default());
    if normalized_query.is_empty() {
        return Json(vec![]);
    }
    let page_size = query.ps.unwrap_or(DEFAULT_AUTOCOMPLETE_PAGE_SIZE).max(1);
    let client = state.client.read().await;
    let autocomplete = client.autocomplete_tags(&normalized_query, page_size);
    let autocomplete = autocomplete.iter().map(|x| {
        let query = normalized_query.clone();
        let tags = x.head.iter().map(|x| x.as_str()).chain(once(x.last.as_str()))
            .map(|x| x.to_string()).collect::<Vec<String>>();
        let suggestion = tags.join(" ");
        let highlighted_suggestion = match suggestion.starts_with(&query) {
            true => query.clone() + "<mark>" + &suggestion[normalized_query.len()..] + "</mark>",
            false => suggestion.clone(),
        };
        AutocompleteObject { query, suggestion, highlighted_suggestion, media_count: x.media_count }
    }).sorted_by_key(|x| x.media_count).rev().collect::<Vec<AutocompleteObject>>();
    Json(autocomplete)
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
    if let Some(media) = client.get_media_by_id(&media_id) {
        let extended_media: ExtendedMedia = media.into();
        HtmlTemplate(MediaPageTemplate { query: normalized_query, page, media: extended_media, media_exists: true })
    } else {
        HtmlTemplate(MediaPageTemplate { query: normalized_query, page, media: ExtendedMedia::default(), media_exists: false })
    }
}

async fn get_random_media(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let client = state.client.read().await;
    match client.get_random_media() {
        Some(media) => HtmlTemplate(MediaPageTemplate { query: "".to_string(), page: 1, media: media.into(), media_exists: true }),
        None => HtmlTemplate(MediaPageTemplate { query: "".to_string(), page: 1, media: ExtendedMedia::default(), media_exists: false })
    }
}

async fn delete_media(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
) -> impl IntoResponse {
    let mut client = state.client.write().await;
    let media_result = client.delete_media(&media_id).await;
    if media_result.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let maybe_media = media_result.unwrap();
    if maybe_media.is_none() {
        return StatusCode::NOT_FOUND;
    }

    StatusCode::OK
}

async fn get_media_thumbnail(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
) -> impl IntoResponse {
    let client = state.client.read().await;
    let thumbnail_path = client.get_thumbnail_path(&media_id);
    drop(client);
    if !thumbnail_path.exists() {
        let mut response = Response::new(Body::from(DEFAULT_THUMBNAIL));
        response.headers_mut().insert("Cache-Control", "no-store".parse().unwrap());
        response.headers_mut().insert("Content-Type", "image/svg+xml".parse().unwrap());
        return response;
    }
    let bytes = std::fs::read(&thumbnail_path).unwrap();
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", "image/png".parse().unwrap());
    response
}

async fn get_algolia_lib() -> impl IntoResponse {
    Response::new(Body::from(ALGOLIA_LIB))
}

async fn get_awesome_cloud_lib() -> impl IntoResponse {
    Response::new(Body::from(AWESOME_CLOUD_LIB))
}

async fn get_htmx_lib() -> impl IntoResponse {
    Response::new(Body::from(HTMX_LIB))
}

async fn get_jquery_lib() -> impl IntoResponse {
    Response::new(Body::from(JQUERY_LIB))
}

async fn get_tailwind_lib() -> impl IntoResponse {
    Response::new(Body::from(TAILWIND_LIB))
}

async fn get_tailwind_ext_lib() -> impl IntoResponse {
    Response::new(Body::from(TAILWIND_EXT_LIB))
}

async fn get_algolia_styles() -> impl IntoResponse {
    Response::new(Body::from(ALGOLIA_STYLES))
}

async fn stream_media(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
) -> impl IntoResponse {
    let client = state.client.read().await;
    let media = client.get_media_by_id(&media_id);
    if media.is_none() {
        return Response::new(Body::empty());
    }
    let maybe_media_path = client.get_media_path(&media_id);
    if maybe_media_path.is_none() {
        return Response::new(Body::empty());
    }
    drop(client);
    let media_path = maybe_media_path.unwrap();
    let bytes = std::fs::read(media_path).unwrap();
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert("Cache-Control", "public, max-age=31536000".parse().unwrap());
    response.headers_mut().insert("Content-Type", media.unwrap().content_type.parse().unwrap());
    response
}

#[derive(Default, Template)]
#[template(path = "upload.html")]
struct UploadTemplate { }

async fn upload_page() -> impl IntoResponse {
    HtmlTemplate(UploadTemplate::default())
}

async fn upload_files(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    while let Ok(file) = read_multipart_file(&mut multipart).await {
        let filename = file.file_name;
        if filename.chars().any(|x| !x.is_ascii_alphanumeric() && x != '.' && x != '-' && x != '_') {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        let data = file.bytes;
        let mut client = state.client.write().await;
        let media_upload_result = client.upload_media(data, filename).await;
        drop(client);
        if media_upload_result.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        let media = media_upload_result.unwrap().safe_unwrap();
        let thumbnail_data = file.preview_bytes;
        let client = state.client.read().await;
        let thumbnail_path = client.get_thumbnail_path(&media.id);
        drop(client);
        if !thumbnail_path.exists() {
            let thumbnail_save_result = std::fs::write(&thumbnail_path, thumbnail_data);
            if thumbnail_save_result.is_err() {
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
    }
    StatusCode::OK
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

async fn read_multipart_file(multipart: &mut Multipart) -> anyhow::Result<MultipartFile> {
    let mut result = MultipartFile::default();
    {
        let file = multipart.next_field().await?.ok_or(anyhow::anyhow!("No file was uploaded"))?;
        result.file_name = file.file_name().unwrap_or_default().to_string();
        result.content_type = file.content_type().unwrap_or_default().to_string();
        result.bytes = file.bytes().await?.to_vec();
    }
    {
        let file = multipart.next_field().await?.ok_or(anyhow::anyhow!("No preview file was uploaded"))?;
        result.preview_file_name = file.file_name().unwrap_or_default().to_string();
        result.preview_content_type = file.content_type().unwrap_or_default().to_string();
        result.preview_bytes = file.bytes().await?.to_vec();
    }
    Ok(result)
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
