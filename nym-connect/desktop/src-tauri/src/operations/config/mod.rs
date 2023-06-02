// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_config_filepath;
use crate::{error::Result, state::State};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn get_config_id(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<String> {
    state.read().await.get_config_id()
}

#[tauri::command]
pub async fn get_config_file_location(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<String> {
    let id = get_config_id(state).await?;
    Ok(default_config_filepath(id).display().to_string())
}
