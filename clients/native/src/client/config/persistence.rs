// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::disk_persistence::CommonClientPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct ClientPaths {
    #[serde(flatten)]
    pub common_paths: CommonClientPaths,
}

impl ClientPaths {
    pub fn new_default<P: AsRef<Path>>(base_data_directory: P) -> Self {
        ClientPaths {
            common_paths: CommonClientPaths::new_base(base_data_directory),
        }
    }
}
