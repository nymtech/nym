use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::{Gateway, MixNode};
use mixnet_contract_common::{GatewayBond, MixNodeBond};
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn bond_gateway(
  gateway: Gateway,
  pledge: Coin,
  owner_signature: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state)
    .bond_gateway(gateway, owner_signature, pledge.try_into()?)
    .await?;
  Ok(())
}

#[tauri::command]
pub async fn unbond_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state).unbond_gateway().await?;
  Ok(())
}

#[tauri::command]
pub async fn unbond_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state).unbond_mixnode().await?;
  Ok(())
}

#[tauri::command]
pub async fn bond_mixnode(
  mixnode: MixNode,
  owner_signature: String,
  pledge: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state)
    .bond_mixnode(mixnode, owner_signature, pledge.try_into()?)
    .await?;
  Ok(())
}

#[tauri::command]
pub async fn update_mixnode(
  profit_margin_percent: u8,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state)
    .update_mixnode_config(profit_margin_percent)
    .await?;
  Ok(())
}

#[tauri::command]
pub async fn mixnode_bond_details(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Option<MixNodeBond>, BackendError> {
  let guard = state.read().await;
  let client = guard.current_client()?;
  let bond = client.nymd.owns_mixnode(client.nymd.address()).await?;
  Ok(bond)
}

#[tauri::command]
pub async fn gateway_bond_details(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Option<GatewayBond>, BackendError> {
  let guard = state.read().await;
  let client = guard.current_client()?;
  let bond = client.nymd.owns_gateway(client.nymd.address()).await?;
  Ok(bond)
}
