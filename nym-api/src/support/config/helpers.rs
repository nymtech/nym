// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::support::config::{default_config_directory, default_data_directory, Config};
use anyhow::Result;
use std::{fs, io};

fn try_upgrade_v1_1_20_config(id: &str) -> Result<()> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current, i.e. 1.1.21+)
    let Ok(old_config) = ConfigV1_1_20::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(());
    };
    info!("It seems the nym-api is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    Ok(updated.save_to_default_location()?)
}

fn init_paths(id: &str) -> io::Result<()> {
    fs::create_dir_all(default_data_directory(id))?;
    fs::create_dir_all(default_config_directory(id))
}

pub(crate) fn initialise_new(id: &str) -> Result<Config> {
    let config = Config::new(id);
    init_paths(id)?;
    crate::coconut::dkg::controller::init_keypair(&config.coconut_signer)?;
    Ok(config)
}

pub(crate) fn try_load_current_config(id: &str) -> Result<Config> {
    try_upgrade_v1_1_20_config(id)?;

    Ok(Config::read_from_default_path(id)?)
}
