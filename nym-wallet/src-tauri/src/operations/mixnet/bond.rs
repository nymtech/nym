use crate::coin::Coin;
use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use crate::{Gateway, MixNode};
use cosmwasm_std::Uint128;
use mixnet_contract_common::{GatewayBond, MixNodeBond};
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::Fee;

#[tauri::command]
pub async fn bond_gateway(
  gateway: Gateway,
  pledge: Coin,
  owner_signature: String,
  fee: Option<Fee>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  let pledge = pledge.into_backend_coin(state.read().await.current_network().denom())?;
  nymd_client!(state)
    .bond_gateway(gateway, owner_signature, pledge, fee)
    .await?;
  Ok(())
}

#[tauri::command]
pub async fn unbond_gateway(
  fee: Option<Fee>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state).unbond_gateway(fee).await?;
  Ok(())
}

#[tauri::command]
pub async fn unbond_mixnode(
  fee: Option<Fee>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state).unbond_mixnode(fee).await?;
  Ok(())
}

#[tauri::command]
pub async fn bond_mixnode(
  mixnode: MixNode,
  owner_signature: String,
  pledge: Coin,
  fee: Option<Fee>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  let pledge = pledge.into_backend_coin(state.read().await.current_network().denom())?;
  nymd_client!(state)
    .bond_mixnode(mixnode, owner_signature, pledge, fee)
    .await?;
  Ok(())
}

#[tauri::command]
pub async fn update_mixnode(
  profit_margin_percent: u8,
  fee: Option<Fee>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  nymd_client!(state)
    .update_mixnode_config(profit_margin_percent, fee)
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

#[tauri::command]
pub async fn get_operator_rewards(
  address: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Uint128, BackendError> {
  Ok(nymd_client!(state).get_operator_rewards(address).await?)
}
