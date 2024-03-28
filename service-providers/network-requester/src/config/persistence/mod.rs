// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_client_core::config::disk_persistence::CommonClientPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod old;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct NetworkRequesterPaths {
    #[serde(flatten)]
    pub common_paths: CommonClientPaths,
}

impl NetworkRequesterPaths {
    pub fn new_base<P: AsRef<Path>>(base_data_directory: P) -> Self {
        let base_dir = base_data_directory.as_ref();

        NetworkRequesterPaths {
            common_paths: CommonClientPaths::new_base(base_dir),
        }
    }
}
