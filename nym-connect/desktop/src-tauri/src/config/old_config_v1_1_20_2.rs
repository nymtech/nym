// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::NymConnectPaths;
use crate::config::{default_config_filepath, Config};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_20_2::CommonClientPathsV1_1_20_2;
use nym_client_core::config::GatewayEndpointConfig;
use nym_config::read_config_from_toml_file;
pub use nym_socks5_client_core::config::old_config_v1_1_20_2::ConfigV1_1_20_2 as CoreConfigV1_1_20_2;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct SocksClientPathsV1_1_20_2 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV1_1_20_2,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_20_2 {
    pub core: CoreConfigV1_1_20_2,

    pub storage_paths: SocksClientPathsV1_1_20_2,

    pub logging: LoggingSettings,
}

impl ConfigV1_1_20_2 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    // in this upgrade, gateway endpoint configuration was moved out of the config file,
    // so its returned to be stored elsewhere.
    pub fn upgrade(self) -> (Config, GatewayEndpointConfig) {
        let gateway_details = self.core.base.client.gateway_endpoint.clone().into();
        let config = Config {
            core: self.core.into(),
            storage_paths: NymConnectPaths {
                common_paths: self.storage_paths.common_paths.upgrade_default(),
            },
            // logging: self.logging,
        };

        (config, gateway_details)
    }
}
