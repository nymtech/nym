// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::default_config_filepath;
use crate::config::old_config_v2::ConfigV2;
use crate::error::IpPacketRouterError;
use log::{info, trace};
use nym_client_core::cli_helpers::CliClientConfig;
use std::path::Path;

async fn try_upgrade_v2_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, IpPacketRouterError> {
    // explicitly load it as v2 (which is incompatible with the current one)
    let Ok(old_config) = ConfigV2::read_from_toml_file(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using v2 config template.");
    info!("It is going to get updated to the current specification.");

    let updated = old_config.try_upgrade()?;

    updated.save_to(config_path)?;
    Ok(true)
}

pub async fn try_upgrade_config<P: AsRef<Path>>(config_path: P) -> Result<(), IpPacketRouterError> {
    trace!("Attempting to upgrade config");
    if try_upgrade_v2_config(config_path).await? {
        return Ok(());
    }

    Ok(())
}

pub async fn try_upgrade_config_by_id(id: &str) -> Result<(), IpPacketRouterError> {
    try_upgrade_config(default_config_filepath(id)).await
}
