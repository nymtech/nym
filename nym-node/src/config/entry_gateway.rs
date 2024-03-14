// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::EntryGatewayPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayConfig {
    pub storage_paths: EntryGatewayPaths,
}

impl EntryGatewayConfig {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        EntryGatewayConfig {
            storage_paths: EntryGatewayPaths::new(data_dir),
        }
    }
}
