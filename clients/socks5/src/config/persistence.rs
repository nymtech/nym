// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::disk_persistence::CommonClientPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct SocksClientPaths {
    #[serde(flatten)]
    pub common_paths: CommonClientPaths,
}

impl SocksClientPaths {
    pub fn new_default<P: AsRef<Path>>(base_data_directory: P) -> Self {
        SocksClientPaths {
            common_paths: CommonClientPaths::new_base(base_data_directory),
        }
    }
}
