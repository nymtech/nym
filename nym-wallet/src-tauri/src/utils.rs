use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::Operation;
use nym_types::currency::CurrencyDenom;
use nym_types::currency::MajorCurrencyAmount;
use nym_wallet_types::app::AppEnv;
use std::sync::Arc;
use tokio::sync::RwLock;

fn get_env_as_option(key: &str) -> Option<String> {
  match ::std::env::var(key) {
    Ok(res) => Some(res),
    Err(_e) => None,
  }
}

#[tauri::command]
pub fn get_env() -> AppEnv {
  AppEnv {
    ADMIN_ADDRESS: get_env_as_option("ADMIN_ADDRESS"),
    SHOW_TERMINAL: get_env_as_option("SHOW_TERMINAL"),
  }
}

#[tauri::command]
pub async fn owns_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
  Ok(
    nymd_client!(state)
      .owns_mixnode(nymd_client!(state).address())
      .await?
      .is_some(),
  )
}

#[tauri::command]
pub async fn owns_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
  Ok(
    nymd_client!(state)
      .owns_gateway(nymd_client!(state).address())
      .await?
      .is_some(),
  )
}

// NOTE: this uses OUTDATED defaults that might have no resemblance with the reality
// as for the actual transaction, the gas cost is being simulated beforehand
#[tauri::command]
pub async fn outdated_get_approximate_fee(
  operation: Operation,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<MajorCurrencyAmount, BackendError> {
  let approximate_fee = operation.default_fee(nymd_client!(state).gas_price());
  let denom: CurrencyDenom = state.read().await.current_network().denom().try_into()?;
  let mut total_fee = MajorCurrencyAmount::zero(&denom);
  for fee in approximate_fee.amount {
    total_fee = total_fee + MajorCurrencyAmount::from_cosmrs_coin(&fee)?;
  }
  Ok(total_fee)
}
