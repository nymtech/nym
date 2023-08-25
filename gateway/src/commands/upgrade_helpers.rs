// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::config::{default_config_filepath, Config};
use crate::error::GatewayError;
use log::info;

pub(super) fn try_upgrade_v1_1_20_config(id: &str) -> Result<(), GatewayError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current, i.e. 1.1.21+)
    let Ok(old_config) = ConfigV1_1_20::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(());
    };
    info!("It seems the gateway is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    updated
        .save_to_default_location()
        .map_err(|err| GatewayError::ConfigSaveFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        })
}
