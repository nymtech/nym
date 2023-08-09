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
    log::trace!("Start connecting");

    let (msg_receiver, exit_status_receiver) = {
        let mut state_w = state.write().await;
        state_w.start_connecting(&window).await?
    };

    // Setup task for checking status
    let state = state.inner().clone();
    tasks::start_disconnect_listener(state.clone(), window.clone(), exit_status_receiver);
    tasks::start_status_listener(state, window.clone(), msg_receiver);

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
    service_provider: Option<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<()> {
    log::trace!("Setting service_provider: {:?}", &service_provider);
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
    gateway: Option<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<()> {
    log::trace!("Setting gateway: {:?}", &gateway);
    let mut guard = state.write().await;
    guard.set_gateway(gateway);
    Ok(())
}
