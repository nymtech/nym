// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use log::info;
use nym_gateway::config::old_config_v1_1_20::ConfigV1_1_20;
use nym_gateway::config::old_config_v1_1_28::ConfigV1_1_28;
use nym_gateway::config::old_config_v1_1_29::ConfigV1_1_29;
use nym_gateway::config::old_config_v1_1_31::ConfigV1_1_31;
use nym_gateway::config::{default_config_filepath, Config};
use nym_gateway::error::GatewayError;

fn try_upgrade_v1_1_20_config(id: &str) -> Result<bool, GatewayError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current, i.e. 1.1.21+)
    let Ok(old_config) = ConfigV1_1_20::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the gateway is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_28 = old_config.into();
    let updated_step2: ConfigV1_1_29 = updated_step1.into();
    let updated_step3: ConfigV1_1_31 = updated_step2.into();
    let updated: Config = updated_step3.into();
    updated
        .save_to_default_location()
        .map_err(|err| GatewayError::ConfigSaveFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        })?;

    Ok(true)
}

fn try_upgrade_v1_1_28_config(id: &str) -> Result<bool, GatewayError> {
    // explicitly load it as v1.1.28 (which is incompatible with the current, i.e. 1.1.29+)
    let Ok(old_config) = ConfigV1_1_28::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the gateway is using <= v1.1.28 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_29 = old_config.into();
    let updated_step2: ConfigV1_1_31 = updated_step1.into();
    let updated: Config = updated_step2.into();
    updated
        .save_to_default_location()
        .map_err(|err| GatewayError::ConfigSaveFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        })?;

    Ok(true)
}

fn try_upgrade_v1_1_29_config(id: &str) -> Result<bool, GatewayError> {
    // explicitly load it as v1.1.29 (which is incompatible with the current, i.e. 1.1.30+)
    let Ok(old_config) = ConfigV1_1_29::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the gateway is using <= v1.1.29 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_31 = old_config.into();
    let updated: Config = updated_step1.into();
    updated
        .save_to_default_location()
        .map_err(|err| GatewayError::ConfigSaveFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        })?;

    Ok(true)
}

fn try_upgrade_v1_1_31_config(id: &str) -> Result<bool, GatewayError> {
    // explicitly load it as v1.1.35 (which is incompatible with the current, i.e. 1.1.36+)
    let Ok(old_config) = ConfigV1_1_31::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the gateway is using <= v1.1.35 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    updated
        .save_to_default_location()
        .map_err(|err| GatewayError::ConfigSaveFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        })?;

    Ok(true)
}

pub(crate) fn try_upgrade_config(id: &str) -> Result<(), GatewayError> {
    if try_upgrade_v1_1_20_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_28_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_29_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_31_config(id)? {
        return Ok(());
    }

    Ok(())
}
