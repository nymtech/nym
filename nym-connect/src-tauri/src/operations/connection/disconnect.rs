use crate::error::Result;
use crate::models::ConnectResult;
use crate::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn start_disconnecting(
    state: tauri::State<'_, Arc<RwLock<State>>>,
    window: tauri::Window<tauri::Wry>,
) -> Result<ConnectResult> {
    log::trace!("Start disconnecting");
    let mut guard = state.write().await;

    guard.start_disconnecting(&window).await?;

    Ok(ConnectResult {
        address: "PLACEHOLDER".to_string(),
    })
}
