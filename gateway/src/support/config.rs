// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::helpers::{try_load_current_config, OverrideConfig};
use crate::config::Config;
use crate::error::GatewayError;

pub(crate) fn build_config<O: Into<OverrideConfig>>(
    id: String,
    override_args: O,
) -> Result<Config, GatewayError> {
    let config = try_load_current_config(&id)?;
    override_args.into().do_override(config)
}
