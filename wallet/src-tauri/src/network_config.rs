// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::state::WalletState;
use nym_wallet_types::network::Network as WalletNetwork;
use nym_wallet_types::network_config::{Validator, ValidatorUrl, ValidatorUrls};

#[tauri::command]
pub async fn get_nyxd_urls(
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<ValidatorUrls, BackendError> {
    let state = state.read().await;
    let urls: Vec<ValidatorUrl> = state.get_nyxd_urls(network).collect();
    Ok(ValidatorUrls { urls })
}

#[tauri::command]
pub async fn get_nym_api_urls(
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<ValidatorUrls, BackendError> {
    let state = state.read().await;
    let urls: Vec<ValidatorUrl> = state.get_api_urls(network).collect();
    Ok(ValidatorUrls { urls })
}

#[tauri::command]
pub async fn get_selected_nyxd_url(
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<Option<String>, BackendError> {
    let state = state.read().await;
    let url = state.get_selected_nyxd_url(&network).map(String::from);
    log::info!("Selected nyxd url for {network}: {:?}", url);
    Ok(url)
}

#[tauri::command]
pub async fn get_default_nyxd_url(
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    let state = state.read().await;
    let url = state.get_default_nyxd_url(&network).map(String::from);
    log::info!("Default nyxd url for {network}: {:?}", url);
    url.ok_or_else(|| BackendError::WalletNoDefaultValidator)
}

#[tauri::command]
pub async fn select_nyxd_url(
    url: &str,
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::debug!("Selecting new nyxd url for {network}: {url}");
    state.write().await.select_nyxd_url(url, network).await?;
    state.read().await.save_config_files()?;
    Ok(())
}

#[tauri::command]
pub async fn reset_nyxd_url(
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::debug!("Resetting nyxd url for {network} to default");
    state.write().await.reset_nyxd_url(network)?;
    state.read().await.save_config_files()?;
    Ok(())
}

#[tauri::command]
pub async fn select_nym_api_url(
    url: &str,
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::debug!("Selecting new  nym api url for {network}: {url}");
    state.write().await.select_nym_api_url(url, network)?;
    Ok(())
}

#[tauri::command]
pub async fn add_validator(
    validator: Validator,
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::debug!("Add validator for {network}: {validator}");
    let url = validator.try_into()?;
    state.write().await.add_validator_url(url, network);
    Ok(())
}

#[tauri::command]
pub async fn remove_validator(
    validator: Validator,
    network: WalletNetwork,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    log::debug!("Remove validator for {network}: {validator}");
    let url = validator.try_into()?;
    state.write().await.remove_validator_url(url, network);
    Ok(())
}

// Update the list of validators by fecthing additional ones remotely. If it fails, just ignore.
#[tauri::command]
pub async fn update_nyxd_urls(state: tauri::State<'_, WalletState>) -> Result<(), BackendError> {
    let mut w_state = state.write().await;
    let _r = w_state.fetch_updated_nyxd_urls().await;
    Ok(())
}
