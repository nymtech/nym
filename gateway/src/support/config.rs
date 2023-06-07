// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::{override_config, try_load_current_config, OverrideConfig};
use crate::config::Config;
use crate::error::GatewayError;

pub(crate) fn build_config<O: Into<OverrideConfig>>(
    id: String,
    override_args: O,
) -> Result<Config, GatewayError> {
    let config = try_load_current_config(&id)?;

    override_config(config, override_args.into())
}
