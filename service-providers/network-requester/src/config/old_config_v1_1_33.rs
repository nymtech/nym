// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::NetworkRequesterPaths;
use crate::config::Config;
use crate::config::{default_config_filepath, Debug, NetworkRequester};
use crate::error::NetworkRequesterError;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_33::CommonClientPathsV1_1_33;
use nym_client_core::config::old_config_v1_1_33::ConfigV1_1_33 as BaseConfigV1_1_33;
use nym_config::read_config_from_toml_file;
use nym_config::serde_helpers::de_maybe_stringified;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct NetworkRequesterPathsV1_1_33 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV1_1_33,

    /// Location of the file containing our allow.list
    pub allowed_list_location: PathBuf,

    /// Location of the file containing our unknown.list
    pub unknown_list_location: PathBuf,

    #[serde(default)]
    pub nr_description: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_33 {
    pub base: BaseConfigV1_1_33,

    #[serde(default)]
    pub network_requester: NetworkRequesterV1_1_33,

    pub storage_paths: NetworkRequesterPathsV1_1_33,

    #[serde(default)]
    pub network_requester_debug: DebugV1_1_33,

    pub logging: LoggingSettings,
}

impl ConfigV1_1_33 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<Config, NetworkRequesterError> {
        Ok(Config {
            base: self.base.into(),
            network_requester: self.network_requester.into(),
            storage_paths: NetworkRequesterPaths {
                common_paths: self.storage_paths.common_paths.upgrade_default()?,
                allowed_list_location: self.storage_paths.allowed_list_location,
                unknown_list_location: self.storage_paths.unknown_list_location,
                nr_description: self.storage_paths.nr_description,
            },
            network_requester_debug: self.network_requester_debug.into(),
            logging: self.logging,
        })
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkRequesterV1_1_33 {
    /// specifies whether this network requester should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// specifies whether this network requester would send anonymized statistics to a statistics aggregator server
    pub enabled_statistics: bool,

    /// in case of enabled statistics, specifies mixnet client address where a statistics aggregator is running
    pub statistics_recipient: Option<String>,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting debug.traffic.disable_main_poisson_packet_distribution = true,
    pub disable_poisson_rate: bool,

    /// Specifies whether this network requester should be using the deprecated allow-list,
    /// as opposed to the new ExitPolicy.
    /// Note: this field will be removed in a near future.
    pub use_deprecated_allow_list: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub upstream_exit_policy_url: Option<Url>,
}

impl From<NetworkRequesterV1_1_33> for NetworkRequester {
    fn from(value: NetworkRequesterV1_1_33) -> Self {
        NetworkRequester {
            open_proxy: value.open_proxy,
            enabled_statistics: value.enabled_statistics,
            statistics_recipient: value.statistics_recipient,
            disable_poisson_rate: value.disable_poisson_rate,
            use_deprecated_allow_list: value.use_deprecated_allow_list,
            upstream_exit_policy_url: value.upstream_exit_policy_url,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugV1_1_33 {
    /// Defines how often the standard allow list should get updated
    #[serde(with = "humantime_serde")]
    pub standard_list_update_interval: Duration,
}

impl From<DebugV1_1_33> for Debug {
    fn from(value: DebugV1_1_33) -> Self {
        Debug {
            standard_list_update_interval: value.standard_list_update_interval,
        }
    }
}

impl Default for DebugV1_1_33 {
    fn default() -> Self {
        DebugV1_1_33 {
            standard_list_update_interval: DEFAULT_STANDARD_LIST_UPDATE_INTERVAL,
        }
    }
}
