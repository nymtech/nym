use crate::coin::{Coin, Denom};
use crate::error::BackendError;
use crate::state::State;
use crate::Operation;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

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
  let client = state.read().await.client()?;
  Ok(client.owns_mixnode(client.address()).await?.is_some())
}

#[tauri::command]
pub async fn owns_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<bool, BackendError> {
  let client = state.read().await.client()?;
  Ok(client.owns_gateway(client.address()).await?.is_some())
}

// NOTE: this uses OUTDATED defaults that might have no resemblance with the reality
// as for the actual transaction, the gas cost is being simulated beforehand
#[tauri::command]
pub async fn get_approximate_fee(
  operation: Operation,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  let approximate_fee = operation.default_fee(client.gas_price());
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
  pub fn new(
    source_address: &str,
    target_address: &str,
    amount: Option<Coin>,
  ) -> DelegationResult {
    DelegationResult {
      source_address: source_address.to_string(),
      target_address: target_address.to_string(),
      amount,
    }
  }
}