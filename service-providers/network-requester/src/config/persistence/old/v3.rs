// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::disk_persistence::old::v3::CommonClientPathsV3;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct NetworkRequesterPathsV3 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV3,

    /// Location of the file containing our allow.list
    pub allowed_list_location: PathBuf,

    /// Location of the file containing our unknown.list
    pub unknown_list_location: PathBuf,

    /// Location of the file containing our description
    pub nr_description: PathBuf,
}
