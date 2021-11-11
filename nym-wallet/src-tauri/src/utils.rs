use crate::coin::{Coin, Denom};
use crate::format_err;
use crate::state::State;
use crate::Operation;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub fn major_to_minor(amount: &str) -> Result<Coin, String> {
  let coin = Coin::new(amount, &Denom::Major);
  Ok(coin.to_minor())
}

#[tauri::command]
pub fn minor_to_major(amount: &str) -> Result<Coin, String> {
  let coin = Coin::new(amount, &Denom::Minor);
  Ok(coin.to_major())
}

#[tauri::command]
pub async fn owns_mixnode(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<bool, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.owns_mixnode(client.address()).await {
    Ok(o) => Ok(o),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn owns_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<bool, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.owns_gateway(client.address()).await {
    Ok(o) => Ok(o),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn get_fee(
  operation: Operation,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  let fee = client.get_fee(operation);
  let mut coin = Coin::new("0", &Denom::Major);
  for f in fee.amount {
    coin = coin + f.into();
  }

  Ok(coin)
}
