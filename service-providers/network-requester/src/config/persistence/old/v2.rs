// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::config::disk_persistence::old_v1_1_33::CommonClientPathsV1_1_33;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct NetworkRequesterPathsV2 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV1_1_33,

    /// Location of the file containing our allow.list
    pub allowed_list_location: PathBuf,

    /// Location of the file containing our unknown.list
    pub unknown_list_location: PathBuf,

    #[serde(default)]
    pub nr_description: PathBuf,
}
