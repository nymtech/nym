// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn simulate_update_contract_settings(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  todo!()
}
