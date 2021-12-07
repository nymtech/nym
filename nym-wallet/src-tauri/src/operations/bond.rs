use crate::coin::Coin;
use crate::format_err;
use crate::state::State;
use crate::{Gateway, MixNode};
use cosmwasm_std::Coin as CosmWasmCoin;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn bond_gateway(
  gateway: Gateway,
  pledge: Coin,
  owner_signature: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  let pledge: CosmWasmCoin = match pledge.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let client = r_state.client()?;
  match client.bond_gateway(gateway, owner_signature, pledge).await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn unbond_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.unbond_gateway().await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn unbond_mixnode(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.unbond_mixnode().await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
pub async fn bond_mixnode(
  mixnode: MixNode,
  owner_signature: String,
  pledge: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  let pledge: CosmWasmCoin = match pledge.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let client = r_state.client()?;
  match client.bond_mixnode(mixnode, owner_signature, pledge).await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
  }
}
