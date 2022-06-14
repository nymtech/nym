use crate::error::BackendError;
use crate::models::ConnectResult;
use crate::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn start_disconnecting(
    state: tauri::State<'_, Arc<RwLock<State>>>,
    window: tauri::Window<tauri::Wry>,
) -> Result<ConnectResult, BackendError> {
    let mut guard = state.write().await;

    guard.start_disconnecting(&window).await;

    Ok(ConnectResult {
        // WIP(JON): fixme
        address: "Test".to_string(),
    })
}
