// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::network::Network as WalletNetwork;
use crate::state::State;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/validatorurls.ts"))]
#[derive(Serialize, Deserialize)]
pub struct ValidatorUrls {
  pub urls: Vec<String>,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/validatorurls.ts"))]
#[derive(Serialize, Deserialize)]
pub struct Validator {
  pub nymd_url: String,
  pub api_url: Option<String>,
}

#[tauri::command]
pub async fn get_validator_nymd_urls(
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ValidatorUrls, BackendError> {
  let state = state.read().await;
  let urls: Vec<String> = state
    .get_nymd_urls(network)
    .map(|url| url.to_string())
    .collect();
  Ok(ValidatorUrls { urls })
}

#[tauri::command]
pub async fn get_validator_api_urls(
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ValidatorUrls, BackendError> {
  let state = state.read().await;
  let urls: Vec<String> = state
    .get_api_urls(network)
    .map(|url| url.to_string())
    .collect();
  Ok(ValidatorUrls { urls })
}

#[tauri::command]
pub async fn select_validator_nymd_url(
  url: &str,
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  state
    .write()
    .await
    .select_validator_nymd_url(url, network)?;
  Ok(())
}

#[tauri::command]
pub async fn select_validator_api_url(
  url: &str,
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  state.write().await.select_validator_api_url(url, network)?;
  Ok(())
}

#[tauri::command]
pub async fn add_validator(
  validator: Validator,
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  let url = validator.try_into()?;
  state.write().await.add_validator_url(url, network);
  Ok(())
}

#[tauri::command]
pub async fn remove_validator(
  validator: Validator,
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  let url = validator.try_into()?;
  state.write().await.remove_validator_url(url, network);
  Ok(())
}

// Update the list of validators by fecthing additional ones remotely. If it fails, just ignore.
#[tauri::command]
pub async fn update_validator_urls(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  let mut w_state = state.write().await;
  let _r = w_state.fetch_updated_validator_urls().await;
  Ok(())
}
