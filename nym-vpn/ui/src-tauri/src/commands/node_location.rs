use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, instrument};
use ts_rs::TS;

use crate::{
    country::DEFAULT_NODE_LOCATION,
    error::{CmdError, CmdErrorSource},
    states::{app::Country, SharedAppData, SharedAppState},
};

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub enum NodeType {
    Entry,
    Exit,
}

#[instrument(skip(app_state, data_state))]
#[tauri::command]
pub async fn set_node_location(
    app_state: State<'_, SharedAppState>,
    data_state: State<'_, SharedAppData>,
    node_type: NodeType,
    country: Country,
) -> Result<(), CmdError> {
    debug!("set_node_location");
    let mut state = app_state.lock().await;
    match node_type {
        NodeType::Entry => {
            state.entry_node_location = Some(country.clone());
        }
        NodeType::Exit => {
            state.exit_node_location = Some(country.clone());
        }
    }

    // save the location on disk
    let mut app_data_store = data_state.lock().await;
    let mut app_data = app_data_store
        .read()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;

    match node_type {
        NodeType::Entry => {
            app_data.entry_node_location = Some(country);
        }
        NodeType::Exit => {
            app_data.exit_node_location = Some(country);
        }
    }
    app_data_store.data = app_data;
    app_data_store
        .write()
        .await
        .map_err(|e| CmdError::new(CmdErrorSource::InternalError, e.to_string()))?;

    Ok(())
}

#[instrument]
#[tauri::command]
pub async fn get_default_node_location() -> Result<Country, CmdError> {
    debug!("get_default_node_location");
    Ok(DEFAULT_NODE_LOCATION.clone())
}
