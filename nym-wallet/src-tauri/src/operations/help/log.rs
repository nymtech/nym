use crate::error::BackendError;
use crate::platform_constants::SECONDARY_LOG_WEBVIEW_SUPPORTED;
use crate::webview_theme::NYM_WALLET_WEBVIEW_BG;
use tauri::webview::PageLoadEvent;
use tauri::Manager;

/// Separate log viewer window is disabled on Windows due to WebView2 freezes / "(Not Responding)".
#[tauri::command]
#[must_use]
pub fn log_viewer_window_supported() -> bool {
    SECONDARY_LOG_WEBVIEW_SUPPORTED
}

#[tauri::command]
pub fn help_log_toggle_window(app_handle: tauri::AppHandle) -> Result<(), BackendError> {
    if let Some(current_log_window) = app_handle.get_webview_window("log") {
        log::info!("Closing log window...");
        if let Err(err) = current_log_window.close() {
            log::error!("Unable to close log window: {err}");
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        log::info!("Log viewer window is disabled on Windows; use stdout or RUST_LOG.");
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        log::info!("Creating log window...");
        match tauri::WebviewWindowBuilder::new(
            &app_handle,
            "log",
            tauri::WebviewUrl::App("log.html".into()),
        )
        .title("Nym Wallet Logs")
        .background_color(NYM_WALLET_WEBVIEW_BG)
        .use_https_scheme(true)
        .on_page_load(|window, payload| match payload.event() {
            PageLoadEvent::Started => {
                log::debug!("Log webview load started: {}", payload.url());
            }
            PageLoadEvent::Finished => {
                log::info!("Log webview load finished: {}", payload.url());
                if std::env::var("NYM_WALLET_LOG_WEBVIEW_DEVTOOLS")
                    .ok()
                    .as_deref()
                    == Some("1")
                {
                    window.open_devtools();
                }
            }
        })
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
}
