use std::fs::File;
use std::io::Write;
use std::iter::once;
use base64::decode;
use itertools::Itertools;
use tauri::State;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use crate::entities::{Media, MediaId};
use crate::utils::normalize_query;
use crate::web_ui::{extract_tags, get_bg_color, get_fg_color, AppState, AutocompleteObject, ExtendedMedia, ExtendedTag, DEFAULT_AUTOCOMPLETE_PAGE_SIZE};

#[tauri::command(rename_all = "snake_case")]
pub async fn choose_files(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let file_paths = app_handle.dialog().file().blocking_pick_files().ok_or("No files selected")?;
    let file_paths = file_paths.iter().map(|p| p.to_string()).collect();
    Ok(file_paths)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn load_media_from_file(path_str: &str, app_state: State<'_, AppState>) -> Result<ExtendedMedia, String> {
    let path_buf = std::path::PathBuf::from(&path_str);
    if !path_buf.exists() {
        return Err("File does not exist".to_string());
    }
    let client = app_state.client.read().await;
    let mut media = client.create_media_from_file(&path_buf).await.map_err(|e| e.to_string())?;
    let maybe_existing_media = client.get_media_by_id(&media.id);
    drop(client);
    if maybe_existing_media.is_none() {
        return Ok(ExtendedMedia::create(media, &app_state.config));
    }
    let mut client = app_state.client.write().await;
    media = client.update_media(media).await.map_err(|e| e.to_string())?.safe_unwrap();
    drop(client);
    let existing_media = maybe_existing_media.unwrap();
    for tag in existing_media.tags {
        media.tags.push(tag.clone());
    }
    Ok(ExtendedMedia::create(media, &app_state.config))
}

#[tauri::command(rename_all = "snake_case")]
pub fn has_thumbnail(media_id: &str, app_state: State<'_, AppState>) -> Result<bool, String> {
    let filepath = app_state.config.thumbnails_dir.join(format!("{}.png", media_id));
    Ok(filepath.exists())
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_thumbnail(media_id: &str, thumbnail: &str, app_state: State<'_, AppState>) -> Result<(), String> {
    let filepath = app_state.config.thumbnails_dir.join(format!("{}.png", media_id));
    if filepath.exists() {
        if filepath.is_dir() {
            return Err("Thumbnail path is a directory".to_string());
        }
        return Ok(());
    }
    let data = thumbnail.split(",").nth(1).ok_or("Invalid data URL")?;
    let bytes = decode(data).map_err(|e| e.to_string())?;
    let mut file = File::create(filepath).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_tag_to_media(media_id: &str, tags: &str, path: Option<&str>, app_state: State<'_, AppState>) -> Result<Vec<ExtendedTag>, String> {
    let media_id: MediaId = media_id.to_string();
    let tags_str = normalize_query(tags);
    let tags_str = tags_str.trim_end();
    if tags_str.is_empty() {
        return Err("No tags provided".to_string());
    }
    let media = get_or_create_media(&media_id, path, &app_state).await?;
    let tags = extract_tags(tags_str);
    let new_tags = tags.iter().filter(|x| !media.tags.contains(x)).cloned().collect::<Vec<String>>();
    if new_tags.is_empty() {
        return Err("No new tags provided".to_string());
    }
    let mut client = app_state.client.write().await;
    for tag in &new_tags {
        client.add_tag_to_media(&media_id, tag).await.unwrap();
    }
    drop(client);
    let added_tags = new_tags.iter().map(|x| {
        let bg_color = get_bg_color(x);
        let fg_color = get_fg_color(&bg_color);
        ExtendedTag { name: x.clone(), is_in_query: false, bg_color, fg_color }
    }).collect::<Vec<ExtendedTag>>();
    Ok(added_tags)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn remove_tag_from_media(media_id: &str, tags: &str, app_state: State<'_, AppState>) -> Result<(), String> {
    let media_id: MediaId = media_id.to_string();
    let tags_str = normalize_query(tags);
    let tags_str = tags_str.trim_end();
    if tags_str.is_empty() {
        return Err("No tags provided".to_string());
    }
    let client = app_state.client.read().await;
    let maybe_media = client.get_media_by_id(&media_id);
    drop(client);

    if maybe_media.is_none() {
        return Err("Media not found".to_string());
    }
    let media = maybe_media.unwrap();
    let tags = extract_tags(tags_str);
    let removed_tags = tags.iter().filter(|x| media.tags.contains(x)).cloned().collect::<Vec<String>>();
    if removed_tags.is_empty() {
        return Err("No tags to remove".to_string());
    }

    let mut client = app_state.client.write().await;
    for tag in &removed_tags {
        client.remove_tag_from_media(&media_id, tag).await.unwrap();
    }
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_media(media_id: &str, app_handle: tauri::AppHandle, app_state: State<'_, AppState>) -> Result<bool, String> {
    let confirmed = app_handle
        .dialog()
        .message("Are you sure you want to delete this media?")
        .title("Confirm deletion")
        .buttons(MessageDialogButtons::OkCancel)
        .blocking_show();
    if !confirmed {
        return Ok(false);
    }

    let media_id: MediaId = media_id.to_string();
    let client = app_state.client.read().await;
    let maybe_media = client.get_media_by_id(&media_id);
    drop(client);

    if maybe_media.is_none() {
        return Err("Media not found".to_string());
    }
    let mut client = app_state.client.write().await;
    client.delete_media(&media_id).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn autocomplete_tags(query: &str, app_state: State<'_, AppState>) -> Result<Vec<AutocompleteObject>, String> {
    let normalized_query = normalize_query(query);
    if normalized_query.is_empty() {
        return Ok(vec![]);
    }
    let page_size = DEFAULT_AUTOCOMPLETE_PAGE_SIZE;
    let client = app_state.client.read().await;
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
    Ok(autocomplete)
}

async fn get_or_create_media(media_id: &MediaId, path: Option<&str>, app_state: &State<'_, AppState>) -> Result<Media, String> {
    let client = app_state.client.read().await;
    let mut maybe_media = client.get_media_by_id(&media_id);
    drop(client);

    if maybe_media.is_some() {
        return Ok(maybe_media.unwrap());
    }

    if path.is_none() {
        return Err("Media not found".to_string());
    }

    let path = path.unwrap();
    let client = app_state.client.read().await;
    maybe_media = client.create_media_from_file(&path.into()).await.ok();
    drop(client);
    if maybe_media.is_none() {
        return Err("Media not found (1)".to_string());
    }

    let mut client = app_state.client.write().await;
    maybe_media = client.add_media(maybe_media.unwrap()).await.ok().map(|x| x.safe_unwrap());
    drop(client);
    if maybe_media.is_none() {
        return Err("Media not found (2)".to_string());
    }

    Ok(maybe_media.unwrap())
}
