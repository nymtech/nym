use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use nym_wallet_types::epoch::Epoch;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn get_current_epoch(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Epoch, BackendError> {
    log::info!(">>> Get curren epoch");
    let interval = nymd_client!(state).get_current_epoch().await?;
    log::info!("<<< curren epoch = {}", interval);
    Ok(interval.into())
}
