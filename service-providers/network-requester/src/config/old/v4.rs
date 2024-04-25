// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_config_filepath;
use crate::config::old::v5::{ConfigV5, DebugV5, NetworkRequesterV5};
use crate::config::persistence::old::v2::NetworkRequesterPathsV2;
use crate::config::persistence::old::v3::NetworkRequesterPathsV3;
use crate::error::NetworkRequesterError;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::old_config_v1_1_33::ConfigV1_1_33 as BaseConfigV1_1_33;
use nym_config::read_config_from_toml_file;
use nym_config::serde_helpers::de_maybe_stringified;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::time::Duration;
use url::Url;

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV4 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_33,

    #[serde(default)]
    pub network_requester: NetworkRequesterV4,

    pub storage_paths: NetworkRequesterPathsV2,

    #[serde(default)]
    pub network_requester_debug: DebugV4,

    pub logging: LoggingSettings,
}

impl ConfigV4 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    #[allow(dead_code)]
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<ConfigV5, NetworkRequesterError> {
        Ok(ConfigV5 {
            base: self.base.into(),
            network_requester: self.network_requester.into(),
            storage_paths: NetworkRequesterPathsV3 {
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
pub struct NetworkRequesterV4 {
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

impl From<NetworkRequesterV4> for NetworkRequesterV5 {
    fn from(value: NetworkRequesterV4) -> Self {
        NetworkRequesterV5 {
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
pub struct DebugV4 {
    /// Defines how often the standard allow list should get updated
    #[serde(with = "humantime_serde")]
    pub standard_list_update_interval: Duration,
}

impl From<DebugV4> for DebugV5 {
    fn from(value: DebugV4) -> Self {
        DebugV5 {
            standard_list_update_interval: value.standard_list_update_interval,
        }
    }
}

impl Default for DebugV4 {
    fn default() -> Self {
        DebugV4 {
            standard_list_update_interval: DEFAULT_STANDARD_LIST_UPDATE_INTERVAL,
        }
    }
}
