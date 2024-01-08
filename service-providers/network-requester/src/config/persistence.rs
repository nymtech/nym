// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_client_core::config::disk_persistence::CommonClientPaths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_ALLOWED_LIST_FILENAME: &str = "allowed.list";
pub const DEFAULT_UNKNOWN_LIST_FILENAME: &str = "unknown.list";
pub const DEFAULT_DESCRIPTION_FILENAME: &str = "description.toml";

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct NetworkRequesterPaths {
    #[serde(flatten)]
    pub common_paths: CommonClientPaths,

    /// Deprecated
    /// Location of the file containing our allow.list
    pub allowed_list_location: PathBuf,

    /// Deprecated
    /// Location of the file containing our unknown.list
    pub unknown_list_location: PathBuf,

    /// Location of the file containing our description
    // For upgrade use default if missing. On next config upgrade iteration, remove the serde(default)
    #[serde(default)]
    pub nr_description: PathBuf,
}

impl NetworkRequesterPaths {
    pub fn new_base<P: AsRef<Path>>(base_data_directory: P) -> Self {
        let base_dir = base_data_directory.as_ref();

        NetworkRequesterPaths {
            common_paths: CommonClientPaths::new_base(base_dir),
            allowed_list_location: base_dir.join(DEFAULT_ALLOWED_LIST_FILENAME),
            unknown_list_location: base_dir.join(DEFAULT_UNKNOWN_LIST_FILENAME),
            nr_description: base_dir.join(DEFAULT_DESCRIPTION_FILENAME),
        }
    }
}
