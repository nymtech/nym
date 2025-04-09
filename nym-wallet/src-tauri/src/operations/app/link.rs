use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn open_url(url: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("Opening URL: {}", url);

    match app_handle.opener().open_url(&url, None::<&str>) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to open URL: {}", err)),
    }
}
