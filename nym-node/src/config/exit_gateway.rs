// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::ExitGatewayPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayConfig {
    pub storage_paths: ExitGatewayPaths,
    // TODO: all NR/IPR things should eventually live here
}

impl ExitGatewayConfig {
    pub fn new_default<P: AsRef<Path>>(config_dir: P) -> Self {
        ExitGatewayConfig {
            storage_paths: ExitGatewayPaths::new(config_dir),
        }
    }
}
