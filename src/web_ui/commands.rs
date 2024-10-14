use std::fs::File;
use std::io::Write;
use base64::decode;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use crate::entities::Media;
use crate::web_ui::AppState;

#[tauri::command(rename_all = "snake_case")]
pub async fn choose_files(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let file_paths = app_handle.dialog().file().blocking_pick_files().ok_or("No files selected")?;
    let file_paths = file_paths.iter().map(|p| p.to_string()).collect();
    Ok(file_paths)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn load_media_from_file(path_str: &str, app_state: State<'_, AppState>) -> Result<Media, String> {
    let path_buf = std::path::PathBuf::from(&path_str);
    if !path_buf.exists() {
        return Err("File does not exist".to_string());
    }
    let client = app_state.client.read().await;
    let mut media = client.create_media_from_file(&path_buf).await.map_err(|e| e.to_string())?;
    let maybe_existing_media = client.get_media_by_id(&media.id);
    drop(client);
    if maybe_existing_media.is_none() {
        return Ok(media);
    }
    let mut client = app_state.client.write().await;
    media = client.update_media(media).await.map_err(|e| e.to_string())?.safe_unwrap();
    drop(client);
    let existing_media = maybe_existing_media.unwrap();
    for tag in existing_media.tags {
        media.tags.push(tag.clone());
    }
    Ok(media)
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