// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::v4::{ConfigV14, DebugV4, NetworkRequesterV4};
use crate::config::persistence::old::v1::NetworkRequesterPathsV1;
use crate::config::persistence::old::v2::NetworkRequesterPathsV2;
use crate::config::persistence::DEFAULT_DESCRIPTION_FILENAME;
use crate::{config::default_config_filepath, error::NetworkRequesterError};
use log::trace;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::old_config_v1_1_20_2::ConfigV1_1_20_2 as BaseClientConfigV1_1_20_2;
use nym_client_core::config::old_config_v1_1_30::ConfigV1_1_30 as BaseConfigV1_1_30;
use nym_client_core::config::old_config_v1_1_33::OldGatewayEndpointConfigV1_1_33;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::time::Duration;

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV3 {
    #[serde(flatten)]
    pub base: BaseClientConfigV1_1_20_2,

    #[serde(default)]
    pub network_requester: NetworkRequesterV3,

    pub storage_paths: NetworkRequesterPathsV1,

    #[serde(default)]
    pub network_requester_debug: DebugV3,

    pub logging: LoggingSettings,
}

impl ConfigV3 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    #[allow(dead_code)]
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    // in this upgrade, gateway endpoint configuration was moved out of the config file,
    // so its returned to be stored elsewhere.
    pub fn upgrade(
        self,
    ) -> Result<(ConfigV14, OldGatewayEndpointConfigV1_1_33), NetworkRequesterError> {
        trace!("Upgrading from v1.1.20_2");
        let gateway_details = self.base.client.gateway_endpoint.clone().into();
        let nr_description = self
            .storage_paths
            .common_paths
            .keys
            .ack_key_file
            .parent()
            .expect("config paths upgrade failure")
            .join(DEFAULT_DESCRIPTION_FILENAME);
        let config = ConfigV14 {
            base: BaseConfigV1_1_30::from(self.base).into(),
            storage_paths: NetworkRequesterPathsV2 {
                common_paths: self.storage_paths.common_paths.upgrade_default()?,
                allowed_list_location: self.storage_paths.allowed_list_location,
                unknown_list_location: self.storage_paths.unknown_list_location,
                nr_description,
            },
            network_requester_debug: self.network_requester_debug.into(),
            logging: self.logging,
            network_requester: self.network_requester.into(),
        };

        Ok((config, gateway_details))
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkRequesterV3 {}

impl From<NetworkRequesterV3> for NetworkRequesterV4 {
    fn from(_value: NetworkRequesterV3) -> Self {
        NetworkRequesterV4::default()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugV3 {
    /// Defines how often the standard allow list should get updated
    #[serde(with = "humantime_serde")]
    pub standard_list_update_interval: Duration,
}

impl From<DebugV3> for DebugV4 {
    fn from(value: DebugV3) -> Self {
        DebugV4 {
            standard_list_update_interval: value.standard_list_update_interval,
        }
    }
}

impl Default for DebugV3 {
    fn default() -> Self {
        DebugV3 {
            standard_list_update_interval: DEFAULT_STANDARD_LIST_UPDATE_INTERVAL,
        }
    }
}
