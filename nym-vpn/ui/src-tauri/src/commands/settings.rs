use tauri::State;
use tracing::{debug, instrument};

use crate::{error::CommandError, fs::data::AppData, states::SharedAppData};

#[instrument]
#[tauri::command]
pub async fn save_user_settings(state: State<'_, SharedAppData>) -> Result<(), CommandError> {
    debug!("save_user_settings");
    let app_data = state.lock().await;
    app_data
        .write()
        .map_err(|e| CommandError::InternalError(e.to_string()))?;

    Ok(())
}

#[instrument]
#[tauri::command]
pub async fn set_user_settings(
    state: State<'_, SharedAppData>,
    settings: AppData,
) -> Result<(), CommandError> {
    debug!("set_user_settings");
    let mut app_data = state.lock().await;
    app_data.data = settings;
    app_data
        .write()
        .map_err(|e| CommandError::InternalError(e.to_string()))?;

    Ok(())
}
