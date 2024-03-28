use std::sync::Arc;

use crate::{error::BackendError, state::State};
use tauri::Manager;
use tokio::sync::RwLock;

#[tauri::command]
pub fn help_clear_storage(app_handle: tauri::AppHandle) -> Result<(), BackendError> {
    log::info!("Clearing user data");

    let state = app_handle.try_state::<Arc<RwLock<State>>>();
    if let Some(s) = state {
        let mut guard = s.blocking_write();
        guard.clear_user_data().ok();
    } else {
        log::warn!("fail to retrieve the state, user data has not been cleared");
    }

    Ok(())
}
