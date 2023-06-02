// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_19::{ConfigV1_1_19, Socks5V1_1_19};
use nym_client_core::config::old_config_v1_1_13::OldConfigV1_1_13 as OldBaseConfigV1_1_13;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use nym_config::must_get_home;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OldConfigV1_1_13 {
    #[serde(flatten)]
    pub base: OldBaseConfigV1_1_13<OldConfigV1_1_13>,

    pub socks5: Socks5V1_1_19,
}

impl MigrationNymConfig for OldConfigV1_1_13 {
    fn default_root_directory() -> PathBuf {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        let base_dir = must_get_home();
        #[cfg(any(target_os = "android", target_os = "ios"))]
        let base_dir = PathBuf::from("/tmp");

        base_dir.join(".nym").join("socks5-clients")
    }
}

impl From<OldConfigV1_1_13> for ConfigV1_1_19 {
    fn from(value: OldConfigV1_1_13) -> Self {
        ConfigV1_1_19 {
            base: value.base.into(),
            socks5: value.socks5,
        }
    }
}
