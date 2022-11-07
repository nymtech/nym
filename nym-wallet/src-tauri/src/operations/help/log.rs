use crate::error::BackendError;
use tauri::Manager;

#[tauri::command]
pub fn help_log_toggle_window(app_handle: tauri::AppHandle) -> Result<(), BackendError> {
    if let Some(current_log_window) = app_handle.windows().get("log") {
        log::info!("Closing log window...");
        if let Err(e) = current_log_window.close() {
            log::error!("Unable to close log window: {:?}", e);
        }
        return Ok(());
    }

    log::info!("Creating log window...");
    match tauri::WindowBuilder::new(&app_handle, "log", tauri::WindowUrl::App("log.html".into()))
        .title("Nym Wallet Logs")
        .build()
    {
        Ok(window) => {
            if let Err(e) = window.set_focus() {
                log::error!("Unable to focus log window: {:?}", e);
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Unable to create log window: {:?}", e);
            Err(BackendError::NewWindowError)
        }
    }
}
