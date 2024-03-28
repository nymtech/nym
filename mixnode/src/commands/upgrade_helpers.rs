// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use log::info;
use nym_mixnode::config::old_config_v1_1_21::ConfigV1_1_21;
use nym_mixnode::config::old_config_v1_1_32::ConfigV1_1_32;
use nym_mixnode::config::{default_config_filepath, Config};
use nym_mixnode::error::MixnodeError;

fn try_upgrade_v1_1_21_config(id: &str) -> Result<bool, MixnodeError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.21 (which is incompatible with the current, i.e. 1.1.22+)
    let Ok(old_config) = ConfigV1_1_21::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the mixnode is using <= v1.1.21 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_32 = old_config.into();

    let updated: Config = updated_step1.into();
    updated
        .save_to_default_location()
        .map_err(|err| MixnodeError::ConfigSaveFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        })?;

    Ok(true)
}

fn try_upgrade_v1_1_32_config(id: &str) -> Result<bool, MixnodeError> {
    // explicitly load it as v1.1.32 (which is incompatible with the current, i.e. 1.1.22+)
    let Ok(old_config) = ConfigV1_1_32::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the mixnode is using <= v1.1.32 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    updated
        .save_to_default_location()
        .map_err(|err| MixnodeError::ConfigSaveFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        })?;

    Ok(true)
}

pub(crate) fn try_upgrade_config(id: &str) -> Result<(), MixnodeError> {
    if try_upgrade_v1_1_21_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_32_config(id)? {
        return Ok(());
    }
    Ok(())
}
