use crate::error::BackendError;
use crate::window::window_hide;
use tauri::{AppHandle, Wry};

#[tauri::command]
pub async fn hide_window(app: AppHandle<Wry>) -> Result<(), BackendError> {
    window_hide(&app);
    Ok(())
}
