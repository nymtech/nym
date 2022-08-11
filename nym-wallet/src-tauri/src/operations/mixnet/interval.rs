use crate::error::BackendError;
use crate::nymd_client;
use crate::state::WalletState;
use nym_wallet_types::interval::Interval;

#[tauri::command]
pub async fn get_current_interval(
    state: tauri::State<'_, WalletState>,
) -> Result<Interval, BackendError> {
    todo!()
    // log::info!(">>> Get current interval");
    // let interval = nymd_client!(state).get_current_interval().await?;
    // log::info!("<<< current interval = {}", interval);
    // Ok(interval.into())
}
