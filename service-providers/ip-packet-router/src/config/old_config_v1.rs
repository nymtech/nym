// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::IpPacketRouterPaths;
use crate::config::Config;
use crate::config::{default_config_filepath, IpPacketRouter};
use crate::error::IpPacketRouterError;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_33::CommonClientPathsV1_1_33;
use nym_client_core::config::old_config_v1_1_33::ConfigV1_1_33 as BaseConfigV1_1_33;
use nym_config::read_config_from_toml_file;
use nym_config::serde_helpers::de_maybe_stringified;
use nym_network_defaults::mainnet;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct IpPacketRouterPathsV1 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV1_1_33,

    /// Location of the file containing our description
    pub ip_packet_router_description: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_33,

    #[serde(default)]
    pub ip_packet_router: IpPacketRouterV1,

    pub storage_paths: IpPacketRouterPathsV1,

    pub logging: LoggingSettings,
}

impl ConfigV1 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<Config, IpPacketRouterError> {
        Ok(Config {
            base: self.base.into(),
            ip_packet_router: self.ip_packet_router.into(),
            storage_paths: IpPacketRouterPaths {
                common_paths: self.storage_paths.common_paths.upgrade_default()?,
                ip_packet_router_description: self.storage_paths.ip_packet_router_description,
            },
            logging: self.logging,
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IpPacketRouterV1 {
    /// Disable Poisson sending rate.
    pub disable_poisson_rate: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub upstream_exit_policy_url: Option<Url>,
}

impl Default for IpPacketRouterV1 {
    fn default() -> Self {
        IpPacketRouterV1 {
            disable_poisson_rate: true,
            upstream_exit_policy_url: Some(
                mainnet::EXIT_POLICY_URL
                    .parse()
                    .expect("invalid default exit policy URL"),
            ),
        }
    }
}

impl From<IpPacketRouterV1> for IpPacketRouter {
    fn from(value: IpPacketRouterV1) -> Self {
        IpPacketRouter {
            disable_poisson_rate: value.disable_poisson_rate,
            upstream_exit_policy_url: value.upstream_exit_policy_url,
        }
    }
}
