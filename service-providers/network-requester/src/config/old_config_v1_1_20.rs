// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::NetworkRequesterPaths;
use crate::config::{default_config_filepath, Config, Debug, NetworkRequester};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_20::CommonClientPathsV1_1_20;
use nym_client_core::config::old_config_v1_1_20::ConfigV1_1_20 as BaseClientConfigV1_1_20;
use nym_client_core::config::GatewayEndpointConfig;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct NetworkRequesterPathsV1_1_20 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV1_1_20,

    /// Location of the file containing our allow.list
    pub allowed_list_location: PathBuf,

    /// Location of the file containing our unknown.list
    pub unknown_list_location: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_20 {
    #[serde(flatten)]
    pub base: BaseClientConfigV1_1_20,

    #[serde(default)]
    pub network_requester: NetworkRequesterV1_1_20,

    pub storage_paths: NetworkRequesterPathsV1_1_20,

    #[serde(default)]
    pub network_requester_debug: DebugV1_1_20,

    pub logging: LoggingSettings,
}

impl ConfigV1_1_20 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    // in this upgrade, gateway endpoint configuration was moved out of the config file,
    // so its returned to be stored elsewhere.
    pub fn upgrade(self) -> (Config, GatewayEndpointConfig) {
        let gateway_details = self.base.client.gateway_endpoint.clone().into();
        let config = Config {
            base: self.base.into(),
            storage_paths: NetworkRequesterPaths {
                common_paths: self.storage_paths.common_paths.upgrade_default(),
                allowed_list_location: self.storage_paths.allowed_list_location,
                unknown_list_location: self.storage_paths.unknown_list_location,
            },
            network_requester_debug: self.network_requester_debug.into(),
            logging: self.logging,
            network_requester: self.network_requester.into(),
        };

        (config, gateway_details)
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkRequesterV1_1_20 {}

impl From<NetworkRequesterV1_1_20> for NetworkRequester {
    fn from(_value: NetworkRequesterV1_1_20) -> Self {
        NetworkRequester {}
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugV1_1_20 {
    /// Defines how often the standard allow list should get updated
    #[serde(with = "humantime_serde")]
    pub standard_list_update_interval: Duration,
}

impl From<DebugV1_1_20> for Debug {
    fn from(value: DebugV1_1_20) -> Self {
        Debug {
            standard_list_update_interval: value.standard_list_update_interval,
        }
    }
}

impl Default for DebugV1_1_20 {
    fn default() -> Self {
        DebugV1_1_20 {
            standard_list_update_interval: DEFAULT_STANDARD_LIST_UPDATE_INTERVAL,
        }
    }
}
