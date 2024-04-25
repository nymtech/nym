// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{default_config_filepath, Config, SocksClientPaths};
use crate::error::Socks5ClientError;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_33::CommonClientPathsV1_1_33;
use nym_config::read_config_from_toml_file;
use nym_socks5_client_core::config::old_config_v1_1_33::ConfigV1_1_33 as CoreConfigV1_1_33;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SocksClientPathsV1_1_33 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV1_1_33,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_33 {
    pub core: CoreConfigV1_1_33,

    // \/ CHANGED
    pub storage_paths: SocksClientPathsV1_1_33,
    // /\ CHANGED
    pub logging: LoggingSettings,
}

impl ConfigV1_1_33 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<Config, Socks5ClientError> {
        Ok(Config {
            core: self.core.into(),
            storage_paths: SocksClientPaths {
                common_paths: self.storage_paths.common_paths.upgrade_default()?,
            },
            logging: self.logging,
        })
    }
}
