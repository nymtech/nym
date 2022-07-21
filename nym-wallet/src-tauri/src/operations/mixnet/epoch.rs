use crate::error::BackendError;
use crate::nymd_client;
use crate::state::WalletState;
use nym_wallet_types::epoch::Epoch;

#[tauri::command]
pub async fn get_current_epoch(
    state: tauri::State<'_, WalletState>,
) -> Result<Epoch, BackendError> {
    log::info!(">>> Get curren epoch");
    let interval = nymd_client!(state).get_current_interval().await?;
    log::info!("<<< curren epoch = {}", interval);
    Ok(interval.into())
}
