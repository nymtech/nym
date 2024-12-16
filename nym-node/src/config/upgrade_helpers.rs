// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::old_configs::*;
use crate::config::Config;
use crate::error::NymNodeError;
use std::path::Path;
use tracing::debug;

// currently there are no upgrades
async fn try_upgrade_config(path: &Path) -> Result<(), NymNodeError> {
    let cfg = try_upgrade_config_v1(path, None).await.ok();
    let cfg = try_upgrade_config_v2(path, cfg).await.ok();
    let cfg = try_upgrade_config_v3(path, cfg).await.ok();
    let cfg = try_upgrade_config_v4(path, cfg).await.ok();
    let cfg = try_upgrade_config_v5(path, cfg).await.ok();
    match try_upgrade_config_v6(path, cfg).await {
        Ok(cfg) => cfg.save(),
        Err(e) => {
            tracing::error!("Failed to finish upgrade - {e}");
            Err(NymNodeError::FailedUpgrade)
        }
    }
}

pub async fn try_load_current_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<Config, NymNodeError> {
    if let Ok(cfg) = Config::read_from_toml_file(config_path.as_ref())
        .inspect_err(|err| debug!("didn't manage to load the current config: {err}"))
    {
        debug!("managed to load the current version of the config");
        return Ok(cfg);
    }

    try_upgrade_config(config_path.as_ref()).await?;
    Config::read_from_toml_file(config_path)
}
