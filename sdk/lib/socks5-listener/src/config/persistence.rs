// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::data_directory_from_root;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct MobileSocksClientPaths {
    #[serde(flatten)]
    pub common_paths: CommonClientPaths,
}

impl MobileSocksClientPaths {
    pub fn new_default<P: AsRef<Path>>(base_data_directory: P) -> Self {
        MobileSocksClientPaths {
            common_paths: CommonClientPaths::new_default(base_data_directory),
        }
    }

    pub fn change_root<P: AsRef<Path>, R: AsRef<Path>>(&mut self, new_root: P, id: R) {
        let new_data_dir = data_directory_from_root(new_root, id);
        self.common_paths = CommonClientPaths::new_default(new_data_dir)
    }
}
