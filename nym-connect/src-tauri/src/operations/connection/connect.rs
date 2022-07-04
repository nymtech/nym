use crate::error::BackendError;
use crate::models::ConnectResult;
use crate::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn start_connecting(
    state: tauri::State<'_, Arc<RwLock<State>>>,
    window: tauri::Window<tauri::Wry>,
) -> Result<ConnectResult, BackendError> {
    let mut guard = state.write().await;

    log::trace!("Start connecting with:");
    log::trace!("  service_provider: {:?}", guard.get_service_provider());
    log::trace!("  gateway: {:?}", guard.get_gateway());
    guard.start_connecting(&window).await;

    Ok(ConnectResult {
        // WIP(JON): fixme
        address: "Test".to_string(),
    })
}

#[tauri::command]
pub async fn get_service_provider(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<String, BackendError> {
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
) -> Result<(), BackendError> {
    log::trace!("Setting service_provider: {service_provider}");
    let mut guard = state.write().await;
    guard.set_service_provider(service_provider);
    Ok(())
}

#[tauri::command]
pub async fn get_gateway(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<String, BackendError> {
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
) -> Result<(), BackendError> {
    log::trace!("Setting gateway: {gateway}");
    let mut guard = state.write().await;
    guard.set_gateway(gateway);
    Ok(())
}
