use crate::error::BackendError;
use crate::nymd_client;
use crate::state::WalletState;
use nym_wallet_types::interval::Interval;
use validator_client::nymd::traits::MixnetQueryClient;

#[tauri::command]
pub async fn get_current_interval(
    state: tauri::State<'_, WalletState>,
) -> Result<Interval, BackendError> {
    log::info!(">>> Get current interval");
    let res = nymd_client!(state).get_current_interval_details().await?;
    log::info!("<<< current interval = {:?}", res);
    Ok(res.interval.into())
}
