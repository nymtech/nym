use tauri::State;
use tracing::{debug, instrument};

use crate::{
    error::{CmdError, CmdErrorSource},
    fs::data::{AppData, UiTheme},
    states::SharedAppData,
};
use crate::fs::data::Country;

#[instrument]
#[tauri::command]
pub async fn get_node_countries(
) -> Result<Vec<Country>, CmdError> {
    debug!("get_node_countries");
    let mut countries : Vec<Country> = Vec::new();
    countries.push(Country{name: "United States", code: "US"});
    countries.push(Country{name: "France", code: "FR"});
    countries.push(Country{name: "Switzerland", code: "CH"});
    countries.push(Country{name: "Sweden", code: "SE"});
    Ok(countries)
}

#[instrument]
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

#[instrument]
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

#[instrument(skip_all)]
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
