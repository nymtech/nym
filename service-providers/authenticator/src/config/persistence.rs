// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::disk_persistence::CommonClientPaths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_DESCRIPTION_FILENAME: &str = "description.toml";

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct AuthenticatorPaths {
    #[serde(flatten)]
    pub common_paths: CommonClientPaths,

    /// Location of the file containing our description
    pub authenticator_description: PathBuf,
}

impl AuthenticatorPaths {
    pub fn new_base<P: AsRef<Path>>(base_data_directory: P) -> Self {
        let base_dir = base_data_directory.as_ref();

        Self {
            common_paths: CommonClientPaths::new_base(base_dir),
            authenticator_description: base_dir.join(DEFAULT_DESCRIPTION_FILENAME),
        }
    }
}
