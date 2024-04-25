// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_config_filepath;
use crate::config::old_config_v1::ConfigV1;
use crate::error::IpPacketRouterError;
use log::{info, trace};
use nym_client_core::cli_helpers::CliClientConfig;
use nym_client_core::client::base_client::storage::migration_helpers::v1_1_33;
use std::path::Path;

async fn try_upgrade_v1_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, IpPacketRouterError> {
    // explicitly load it as v1 (which is incompatible with the current one)
    let Ok(old_config) = ConfigV1::read_from_toml_file(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using v1 config template.");
    info!("It is going to get updated to the current specification.");

    let old_paths = old_config.storage_paths.clone();
    let updated = old_config.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        None,
    )
    .await?;

    updated.save_to(config_path)?;
    Ok(true)
}

pub async fn try_upgrade_config<P: AsRef<Path>>(config_path: P) -> Result<(), IpPacketRouterError> {
    trace!("Attempting to upgrade config");
    if try_upgrade_v1_config(config_path).await? {
        return Ok(());
    }

    Ok(())
}

pub async fn try_upgrade_config_by_id(id: &str) -> Result<(), IpPacketRouterError> {
    try_upgrade_config(default_config_filepath(id)).await
}
