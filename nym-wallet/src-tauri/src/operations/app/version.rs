// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use nym_wallet_types::app::AppVersion;
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub async fn check_version(handle: tauri::AppHandle) -> Result<AppVersion, BackendError> {
    log::info!(">>> Getting app version info");

    let updater = handle.updater().map_err(|e| {
        log::error!("Failed to get updater: {e}");
        BackendError::CheckAppVersionError
    })?;

    // Then check for updates
    let update_info = match updater.check().await {
        Ok(info) => info,
        Err(e) => {
            let msg = e.to_string();
            // Hosted static JSON must include per-platform `signature` (base64) alongside `url`.
            // Legacy manifests with only `url` fail serde in tauri-plugin-updater 2.x.
            if msg.contains("missing field") && msg.contains("signature") {
                let current_version = handle.package_info().version.to_string();
                log::warn!(
                    "Updater check skipped: manifest at configured endpoint is not Tauri 2-compatible \
                     (missing or invalid `signature` field). Users will not be notified of updates \
                     until the hosted updater.json is republished. Error: {msg}"
                );
                return Ok(AppVersion {
                    current_version: current_version.clone(),
                    latest_version: current_version,
                    is_update_available: false,
                });
            }
            log::error!("An error occurred while checking for app update {e}");
            return Err(BackendError::CheckAppVersionError);
        }
    };

    // Process the result
    if let Some(update) = update_info {
        log::debug!(
            "<<< update available: [true], current version {}, latest version {}",
            update.current_version,
            update.version
        );
        Ok(AppVersion {
            current_version: update.current_version.to_string(),
            latest_version: update.version,
            is_update_available: true,
        })
    } else {
        // No update available
        let current_version = handle.package_info().version.to_string();
        log::debug!("<<< update available: [false], current version {current_version}");
        Ok(AppVersion {
            current_version: current_version.clone(),
            latest_version: current_version,
            is_update_available: false,
        })
    }
}
