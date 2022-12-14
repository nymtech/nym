use crate::error::BackendError;
use tauri::Manager;

#[tauri::command]
pub fn help_log_toggle_window(app_handle: tauri::AppHandle) -> Result<(), BackendError> {
    if let Some(current_log_window) = app_handle.windows().get("log") {
        log::info!("Closing log window...");
        if let Err(err) = current_log_window.close() {
            log::error!("Unable to close log window: {err}");
        }
        return Ok(());
    }

    log::info!("Creating log window...");
    match tauri::WindowBuilder::new(&app_handle, "log", tauri::WindowUrl::App("log.html".into()))
        .title("Nym Wallet Logs")
        .build()
    {
        Ok(window) => {
            if let Err(err) = window.set_focus() {
                log::error!("Unable to focus log window: {err}");
            }
            Ok(())
        }
        Err(err) => {
            log::error!("Unable to create log window: {err}");
            Err(BackendError::NewWindowError)
        }
    }
}
