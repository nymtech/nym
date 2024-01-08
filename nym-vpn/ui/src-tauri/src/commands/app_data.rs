use tauri::State;
use tracing::{debug, instrument};

use crate::country::COUNTRIES;
use crate::states::app::Country;
use crate::{
    error::{CmdError, CmdErrorSource},
    fs::data::{AppData, UiTheme},
    states::SharedAppData,
};

#[instrument]
#[tauri::command]
pub fn get_node_countries() -> Result<Vec<Country>, CmdError> {
    debug!("get_node_countries");
    // TODO fetch the list of countries from some API
    Ok(COUNTRIES.clone())
}

#[instrument(skip(state))]
#[tauri::command]
pub async fn set_app_data(
    state: State<'_, SharedAppData>,
    data: Option<AppData>,
) -> Result<(), CmdError> {
    debug!("set_app_data");
    let mut app_data_store = state.lock().await;
    if let Some(data) = data {
        app_data_store.data = data;
    }
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;

    Ok(())
}

#[instrument(skip_all)]
#[tauri::command]
pub async fn get_app_data(
    state: State<'_, SharedAppData>,
    data: Option<AppData>,
) -> Result<AppData, CmdError> {
    debug!("get_app_data");
    let mut app_data_store = state.lock().await;
    if let Some(data) = data {
        app_data_store.data = data;
    }
    let data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;

    Ok(data)
}

#[instrument(skip(data_state))]
#[tauri::command]
pub async fn set_ui_theme(
    data_state: State<'_, SharedAppData>,
    theme: UiTheme,
) -> Result<(), CmdError> {
    debug!("set_ui_theme");

    // save the selected UI theme to disk
    let mut app_data_store = data_state.lock().await;
    let mut app_data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    app_data.ui_theme = Some(theme);
    app_data_store.data = app_data;
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    Ok(())
}

#[instrument(skip(data_state))]
#[tauri::command]
pub async fn set_root_font_size(
    data_state: State<'_, SharedAppData>,
    size: u32,
) -> Result<(), CmdError> {
    debug!("set_root_font_size");

    let mut app_data_store = data_state.lock().await;
    let mut app_data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    app_data.ui_root_font_size = Some(size);
    app_data_store.data = app_data;
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    Ok(())
}

#[instrument(skip(data_state))]
#[tauri::command]
pub async fn set_entry_location_selector(
    data_state: State<'_, SharedAppData>,
    entry_selector: bool,
) -> Result<(), CmdError> {
    debug!("set_entry_location_selector");

    let mut app_data_store = data_state.lock().await;
    let mut app_data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    app_data.entry_location_selector = Some(entry_selector);
    app_data_store.data = app_data;
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    Ok(())
}

#[instrument(skip(data_state))]
#[tauri::command]
pub async fn set_auto_connect(
    data_state: State<'_, SharedAppData>,
    entry_selector: bool,
) -> Result<(), CmdError> {
    debug!("set_auto_connect");

    let mut app_data_store = data_state.lock().await;
    let mut app_data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    app_data.autoconnect = Some(entry_selector);
    app_data_store.data = app_data;
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    Ok(())
}

#[instrument(skip(data_state))]
#[tauri::command]
pub async fn set_monitoring(
    data_state: State<'_, SharedAppData>,
    entry_selector: bool,
) -> Result<(), CmdError> {
    debug!("set_monitoring");

    let mut app_data_store = data_state.lock().await;
    let mut app_data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    app_data.monitoring = Some(entry_selector);
    app_data_store.data = app_data;
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;
    Ok(())
}
