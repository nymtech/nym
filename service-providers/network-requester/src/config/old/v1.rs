// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_20::ConfigV2;
use nym_client_core::config::old_config_v1_1_13::OldConfigV1_1_13 as OldBaseConfigV1_1_13;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OldConfigV1 {
    #[serde(flatten)]
    pub base: OldBaseConfigV1_1_13<OldConfigV1>,
}

impl MigrationNymConfig for OldConfigV1 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("../../../..")
            .join("network-requester")
    }
}

impl From<OldConfigV1> for ConfigV2 {
    fn from(value: OldConfigV1) -> Self {
        ConfigV2 {
            base: value.base.into(),
            network_requester: Default::default(),
            network_requester_debug: Default::default(),
        }
    }
}
