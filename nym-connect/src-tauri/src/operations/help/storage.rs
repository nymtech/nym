use crate::error::BackendError;
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Serialize, Clone)]
struct ClearStorageEvent {
    kind: String,
}

#[tauri::command]
pub fn help_clear_storage(app_handle: tauri::AppHandle) -> Result<(), BackendError> {
    log::info!("Sending event to clear local storage...");

    let event = ClearStorageEvent {
        kind: "local_storage".to_string(),
    };
    app_handle.emit_all("help://clear-storage", event)?;

    Ok(())
}
