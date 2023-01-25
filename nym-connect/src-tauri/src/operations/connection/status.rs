use crate::error::Result;
use crate::tasks;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::models::{ConnectionStatusKind, ConnectivityTestResult, GatewayConnectionStatusKind};
use crate::state::State;

#[tauri::command]
pub async fn get_connection_status(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ConnectionStatusKind> {
    let state = state.read().await;
    Ok(state.get_status())
}

#[tauri::command]
pub async fn get_gateway_connection_status(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<GatewayConnectionStatusKind> {
    let mut state_w = state.write().await;
    let gateway_connectivity = state_w.get_gateway_connectivity();
    Ok(gateway_connectivity.into())
}

#[tauri::command]
pub async fn get_connection_health_check_status(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ConnectivityTestResult> {
    let state = state.read().await;
    Ok(state.get_connectivity_test_result())
}

// Start a connection check task. This should return with an event within one minute, and update
// the state.
// Trying to run multiple concurrent connection checks probably works but is not supported.
#[tauri::command]
pub fn start_connection_health_check_task(
    state: tauri::State<'_, Arc<RwLock<State>>>,
    window: tauri::Window<tauri::Wry>,
) {
    tasks::start_connection_check(state.inner().clone(), window);
}
