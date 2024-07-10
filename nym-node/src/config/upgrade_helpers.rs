// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use tracing::debug;

use crate::config::old_configs::*;
use crate::config::Config;
use crate::error::NymNodeError;
use std::path::Path;

// currently there are no upgrades
async fn try_upgrade_config(path: &Path) -> Result<(), NymNodeError> {
    if try_upgrade_config_1_1_2(path).await.is_ok() {
        debug!("Updated from 1.1.2 or previous");
    }
    if try_upgrade_config_1_1_3(path).await.is_ok() {
        debug!("Updated from 1.1.3");
    }

    Ok(())
}

pub async fn try_load_current_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<Config, NymNodeError> {
    if let Ok(cfg) = Config::read_from_toml_file(config_path.as_ref()) {
        return Ok(cfg);
    }

    try_upgrade_config(config_path.as_ref()).await?;
    Config::read_from_toml_file(config_path)
}
