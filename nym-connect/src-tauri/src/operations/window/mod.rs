use crate::window::window_hide;
use tauri::{AppHandle, Wry};

#[tauri::command]
pub fn hide_window(app: AppHandle<Wry>) {
    window_hide(&app);
}
