use crate::coin::Coin;
use crate::error::BackendError;
use crate::state::State;
use crate::{Gateway, MixNode};
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
  let client = state.read().await.client()?;
  client.bond_gateway(gateway, owner_signature, pledge.try_into()?).await?;
  Ok(())
}

#[tauri::command]
pub async fn unbond_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), BackendError> {
  let client = state.read().await.client()?;
  client.unbond_gateway().await?;
  Ok(())
}

#[tauri::command]
pub async fn unbond_mixnode(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), BackendError> {
  let client = state.read().await.client()?;
  client.unbond_mixnode().await?;
  Ok(())
}

#[tauri::command]
pub async fn bond_mixnode(
  mixnode: MixNode,
  owner_signature: String,
  pledge: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  let client = state.read().await.client()?; 
  client.bond_mixnode(mixnode, owner_signature, pledge.try_into()?).await?;
  Ok(())
}
