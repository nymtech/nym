use tauri::api::notification::Notification;
use nymvpn_config::config;

#[tauri::command]
pub async fn send_desktop_notification(
    app_handle: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    Notification::new(&app_handle.config().tauri.bundle.identifier)
        .title(title)
        .body(body.as_str())
        .icon(config().icon_path())
        .show()
        .map_err(|e| {
            log::error!("failed to send desktop notification: {body}: {e}");
            format!("failed to send desktop notification")
        })
}
