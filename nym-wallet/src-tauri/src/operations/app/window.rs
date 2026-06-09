use tauri::Manager;

use crate::error::BackendError;
use crate::webview_theme::NYM_WALLET_WEBVIEW_BG;

struct WindowGeometry {
    inner_width: f64,
    inner_height: f64,
    outer_x: f64,
    outer_y: f64,
}

fn capture_window_geometry(app_handle: &tauri::AppHandle, label: &str) -> Option<WindowGeometry> {
    let window = app_handle.get_webview_window(label)?;
    let scale_factor = window.scale_factor().ok()?;
    let size = window.inner_size().ok()?;
    let position = window.outer_position().ok()?;

    Some(WindowGeometry {
        inner_width: size.width as f64 / scale_factor,
        inner_height: size.height as f64 / scale_factor,
        outer_x: position.x as f64 / scale_factor,
        outer_y: position.y as f64 / scale_factor,
    })
}

#[tauri::command]
pub async fn create_main_window(app_handle: tauri::AppHandle) -> Result<(), BackendError> {
    // first, try close the sign up/sign in (`main` => `index.html`)
    // then, create the main app window (`nymWalletApp` => `main.html`)
    // see `webpack.common.js` for the `.tsx` file associated with the bundled output entry point used in `new_window_url`
    create_window(app_handle, "nymWalletApp", "main.html", "main").await
}

#[tauri::command]
pub async fn create_auth_window(app_handle: tauri::AppHandle) -> Result<(), BackendError> {
    // first, try close the main app window (`nymWalletApp` => `main.html`)
    // then, create the sign up/sign in (`main` => `index.html`) so the user can log in again
    // see `webpack.common.js` for the `.tsx` file associated with the bundled output entry point used in `new_window_url`
    create_window(app_handle, "main", "index.html", "nymWalletApp").await
}

async fn create_window(
    app_handle: tauri::AppHandle,
    new_window_label: &str,
    new_window_url: &str,
    try_close_window_label: &str,
) -> Result<(), BackendError> {
    let prior_geometry = capture_window_geometry(&app_handle, try_close_window_label);

    // create the new window first, to stop the app process from exiting
    log::info!("Creating {new_window_label} window...");
    let mut builder = tauri::WebviewWindowBuilder::new(
        &app_handle,
        new_window_label,
        tauri::WebviewUrl::App(new_window_url.into()),
    )
    .title("Nym Wallet")
    .background_color(NYM_WALLET_WEBVIEW_BG)
    .use_https_scheme(true);

    if let Some(geometry) = &prior_geometry {
        builder = builder
            .visible(false)
            .inner_size(geometry.inner_width, geometry.inner_height)
            .position(geometry.outer_x, geometry.outer_y);
    }

    match builder.build() {
        Ok(window) => {
            if prior_geometry.is_some() {
                if let Err(err) = window.show() {
                    log::error!("Unable to show window: {err}");
                }
            }
            if let Err(err) = window.set_focus() {
                log::error!("Unable to focus window: {err}");
            }
        }
        Err(err) => {
            log::error!("Unable to create window: {err}");
            return Err(BackendError::NewWindowError);
        }
    }

    // close the old window
    match app_handle.get_webview_window(try_close_window_label) {
        Some(try_close_window) => {
            if let Err(err) = try_close_window.close() {
                log::error!("Could not close window: {err}")
            }
        }
        None => {
            log::error!("Unable to close window `{try_close_window_label}`")
        }
    }

    Ok(())
}
