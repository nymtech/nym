// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::config::default_config_filepath;
use crate::config::old::v6::ConfigV6;
use crate::config::old_config_v1_1_54::ConfigV5;
use crate::error::NetworkRequesterError;
use log::{info, trace};
use nym_client_core::cli_helpers::CliClientConfig;
use std::path::Path;

async fn try_upgrade_v5_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    // explicitly load it as v5 (which is incompatible with the current one)
    let Ok(old_config) = ConfigV5::read_from_toml_file(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v5 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV6 = old_config.into();
    let updated: Config = updated_step1.into();
    updated.save_to(config_path)?;

    Ok(true)
}

async fn try_upgrade_v6_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    // explicitly load it as v6 (which is incompatible with the current one)
    let Ok(old_config) = ConfigV6::read_from_toml_file(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v6 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    updated.save_to(config_path)?;

    Ok(true)
}

pub async fn try_upgrade_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<(), NetworkRequesterError> {
    trace!("Attempting to upgrade config");
    if try_upgrade_v5_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v6_config(config_path).await? {
        return Ok(());
    }

    Ok(())
}

pub async fn try_upgrade_config_by_id(id: &str) -> Result<(), NetworkRequesterError> {
    try_upgrade_config(default_config_filepath(id)).await
}
