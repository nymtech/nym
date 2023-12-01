// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config::old_config_v1_1_21::ConfigV1_1_21;
use crate::support::config::old_config_v1_1_27::ConfigV1_1_27;
use crate::support::config::{default_config_directory, default_data_directory, Config};
use anyhow::Result;
use std::{fs, io};

fn try_upgrade_v1_1_21_config(id: &str) -> Result<()> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.21 (which is incompatible with the current, i.e. 1.1.22+)
    let Ok(old_config) = ConfigV1_1_21::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(());
    };
    info!("It seems the nym-api is using <= v1.1.21 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    crate::network_monitor::init_ecash_keypair(&updated.network_monitor)?; //SW does that belong here?
    Ok(updated.save_to_default_location()?)
}

fn try_upgrade_v1_1_27_config(id: &str) -> Result<()> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.27 (which is incompatible with the current, i.e. 1.1.28+)
    let Ok(old_config) = ConfigV1_1_27::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(());
    };
    info!("It seems the nym-api is using <= v1.1.27 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    crate::network_monitor::init_ecash_keypair(&updated.network_monitor)?; //SW does that belong here?
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
    crate::network_monitor::init_ecash_keypair(&config.network_monitor)?;
    Ok(config)
}

pub(crate) fn try_load_current_config(id: &str) -> Result<Config> {
    try_upgrade_v1_1_21_config(id)?;
    try_upgrade_v1_1_27_config(id)?;

    Ok(Config::read_from_default_path(id)?)
}
