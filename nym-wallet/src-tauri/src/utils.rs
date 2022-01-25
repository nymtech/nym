use crate::coin::{Coin, Denom};
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::Operation;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub fn major_to_minor(amount: &str) -> Coin {
  let coin = Coin::new(amount, &Denom::Major);
  coin.to_minor()
}

#[tauri::command]
pub fn minor_to_major(amount: &str) -> Coin {
  let coin = Coin::new(amount, &Denom::Minor);
  coin.to_major()
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
) -> Result<Coin, BackendError> {
  let approximate_fee = operation.default_fee(nymd_client!(state).gas_price());
  let mut coin = Coin::new("0", &Denom::Major);
  for f in approximate_fee.amount {
    coin = coin + f.into();
  }
  Ok(coin)
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize)]
pub struct DelegationResult {
  source_address: String,
  target_address: String,
  amount: Option<Coin>,
}

impl DelegationResult {
  pub fn new(source_address: &str, target_address: &str, amount: Option<Coin>) -> DelegationResult {
    DelegationResult {
      source_address: source_address.to_string(),
      target_address: target_address.to_string(),
      amount,
    }
  }
}
