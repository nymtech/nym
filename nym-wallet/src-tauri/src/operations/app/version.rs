// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use nym_wallet_types::app::AppVersion;

#[tauri::command]
pub async fn check_version(handle: tauri::AppHandle) -> Result<AppVersion, BackendError> {
    log::info!(">>> Getting app version info");
    let res = tauri::updater::builder(handle)
        .check()
        .await
        .map(|u| AppVersion {
            current_version: u.current_version().to_string(),
            latest_version: u.latest_version().to_owned(),
            is_update_available: u.is_update_available(),
        })
        .map_err(|e| {
            log::error!("An error ocurred while checking for app update {}", e);
            BackendError::CheckAppVersionError
        })?;
    log::debug!(
        "<<< update available: [{}], current version {}, latest version {}",
        res.is_update_available,
        res.current_version,
        res.latest_version
    );
    Ok(res)
}
