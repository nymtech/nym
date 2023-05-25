// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::{override_config, OverrideConfig};
use crate::config::{default_config_filepath, Config};
use crate::error::GatewayError;
use log::error;

pub(crate) fn build_config<O: Into<OverrideConfig>>(
    id: String,
    override_args: O,
) -> Result<Config, GatewayError> {
    let config = match Config::read_from_default_path(&id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(
                "Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})",
            );
            return Err(GatewayError::ConfigLoadFailure {
                path: default_config_filepath(&id),
                id,
                source: err,
            });
        }
    };

    override_config(config, override_args.into())
}
