use std::fs::File;
use std::io::Write;
use base64::decode;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use crate::entities::Media;
use crate::web_ui::AppState;

#[tauri::command(rename_all = "snake_case")]
pub async fn choose_file(app_handle: tauri::AppHandle, app_state: State<'_, AppState>) -> Result<Media, String> {
    let path_str = app_handle.dialog().file().blocking_pick_file().ok_or("No file selected")?.to_string();
    let path_buf = std::path::PathBuf::from(&path_str);
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
pub fn save_thumbnail(image_data_url: &str) -> Result<(), String> {
    let data = image_data_url.split(",").nth(1).ok_or("Invalid data URL")?;
    let bytes = decode(data).map_err(|e| e.to_string())?;
    let mut file = File::create("thumbnail.png").map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(())
}