// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::old_config_v1_1_20::ConfigV1_1_20;
use nym_client_core::config::old_config_v1_1_13::OldConfigV1_1_13 as OldBaseConfigV1_1_13;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OldConfigV1_1_13 {
    #[serde(flatten)]
    pub base: OldBaseConfigV1_1_13<OldConfigV1_1_13>,
}

impl MigrationNymConfig for OldConfigV1_1_13 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("service-providers")
            .join("network-requester")
    }
}

impl From<OldConfigV1_1_13> for ConfigV1_1_20 {
    fn from(value: OldConfigV1_1_13) -> Self {
        ConfigV1_1_20 {
            base: value.base.into(),
            network_requester: Default::default(),
            network_requester_debug: Default::default(),
        }
    }
}
