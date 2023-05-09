use tauri::Manager;

use crate::error::BackendError;

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
    // create the new window first, to stop the app process from exiting
    log::info!("Creating {} window...", new_window_label);
    match tauri::WindowBuilder::new(
        &app_handle,
        new_window_label,
        tauri::WindowUrl::App(new_window_url.into()),
    )
    .title("Nym Wallet")
    .build()
    {
        Ok(window) => {
            if let Err(err) = window.set_focus() {
                log::error!("Unable to focus log window: {err}");
            }
            if let Err(err) = window.maximize() {
                log::error!("Could not maximize window: {err}");
            }
        }
        Err(err) => {
            log::error!("Unable to create log window: {err}");
            return Err(BackendError::NewWindowError);
        }
    }

    // close the old window
    match app_handle.windows().get(try_close_window_label) {
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
