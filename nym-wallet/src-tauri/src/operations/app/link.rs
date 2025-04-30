use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn open_url(url: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("Opening URL: {}", url);

    #[cfg(target_os = "windows")]
    {
        // Windows needs shell capability
        match app_handle.shell().open("", &url) {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("Failed to open URL: {}", err)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // macOS and Linux work well with opener
        match app_handle.opener().open_url(&url, None::<&str>) {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("Failed to open URL: {}", err)),
        }
    }
}
