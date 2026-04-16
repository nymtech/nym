use tauri_plugin_opener::OpenerExt;
use url::Url;

#[tauri::command]
pub async fn open_url(url: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;
    match parsed.scheme() {
        "https" | "http" => {}
        other => {
            return Err(format!("URL scheme not allowed: {other}"));
        }
    }

    match app_handle.opener().open_url(&url, None::<&str>) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to open URL: {err}")),
    }
}
