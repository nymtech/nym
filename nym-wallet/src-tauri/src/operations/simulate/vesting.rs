// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn simulate_vesting_bond_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_vesting_bond_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_vesting_unbond_gateway(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_vesting_unbond_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_vesting_update_mixnode(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}

#[tauri::command]
pub async fn simulate_withdraw_vested_coins(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}
