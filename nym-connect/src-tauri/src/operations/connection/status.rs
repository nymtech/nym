use crate::error::Result;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::models::ConnectionStatusKind;
use crate::state::State;

#[tauri::command]
pub async fn get_connection_status(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ConnectionStatusKind> {
    let state = state.read().await;
    Ok(state.get_status())
}
