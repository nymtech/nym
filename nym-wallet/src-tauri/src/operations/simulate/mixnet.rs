// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn simulate_bond_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
    
    /*
    let guard = state.read().await;

  let to_address = AccountId::from_str(address)?;
  let network_denom = guard.current_network().denom();
  let amount = vec![amount.clone().into_cosmos_coin(&network_denom)?];

  let client = guard.current_client()?;
  let from_address = client.nymd.address().clone();
  let gas_price = client.nymd.gas_price().clone();

  // TODO: I'm still not 100% convinced whether this should be exposed here or handled somewhere else in the client code
  let msg = MsgSend {
    from_address,
    to_address,
    amount,
  };

  let result = client.nymd.simulate(vec![msg]).await?;
  Ok(SimulateResult::new(result.gas_info, gas_price).detailed_fee())
     */
}

#[tauri::command]
pub async fn simulate_bond_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_unbond_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_unbond_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_update_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_delegate_to_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_undelegate_from_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}
