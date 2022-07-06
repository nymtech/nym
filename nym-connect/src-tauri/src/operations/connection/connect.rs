use crate::{
    error::{BackendError, Result},
    models::ConnectResult,
    tasks, State,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn start_connecting(
    state: tauri::State<'_, Arc<RwLock<State>>>,
    window: tauri::Window<tauri::Wry>,
) -> Result<ConnectResult> {
    let status_receiver = {
        let mut state_w = state.write().await;
        state_w.start_connecting(&window).await?
    };

    // Setup task for checking status
    let state = state.inner().clone();
    tasks::start_disconnect_listener(state, window, status_receiver);

    Ok(ConnectResult {
        address: "PLACEHOLDER".to_string(),
    })
}

#[tauri::command]
pub async fn get_service_provider(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<String> {
    let guard = state.read().await;
    guard
        .get_service_provider()
        .clone()
        .ok_or(BackendError::NoServiceProviderSet)
}

#[tauri::command]
pub async fn set_service_provider(
    service_provider: String,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<()> {
    log::trace!("Setting service_provider: {service_provider}");
    let mut guard = state.write().await;
    guard.set_service_provider(service_provider);
    Ok(())
}

#[tauri::command]
pub async fn get_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<String> {
    let guard = state.read().await;
    guard
        .get_gateway()
        .clone()
        .ok_or(BackendError::NoGatewaySet)
}

#[tauri::command]
pub async fn set_gateway(
    gateway: String,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<()> {
    log::trace!("Setting gateway: {gateway}");
    let mut guard = state.write().await;
    guard.set_gateway(gateway);
    Ok(())
}
