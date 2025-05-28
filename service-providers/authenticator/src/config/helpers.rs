// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::{info, trace};
use std::path::Path;

use crate::{config::old_config_v1_1_54::ConfigV1_1_54, error::AuthenticatorError};

async fn try_upgrade_v1_1_54_config<P: AsRef<Path>>(id: P) -> Result<bool, AuthenticatorError> {
    // explicitly load it as v1.1.54 (which is incompatible with the current one, i.e. +1.1.55)
    let Ok(old_config) = ConfigV1_1_54::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.54 config template.");
    info!("It is going to get updated to the current specification.");

    let updated = old_config.try_upgrade()?;

    updated.save_to_default_location()?;
    Ok(true)
}

pub async fn try_upgrade_config<P: AsRef<Path>>(id: P) -> Result<(), AuthenticatorError> {
    trace!("Attempting to upgrade config");
    if try_upgrade_v1_1_54_config(id).await? {
        return Ok(());
    }

    Ok(())
}
