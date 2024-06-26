// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::old::v3::NetworkRequesterPathsV3;
use crate::config::persistence::NetworkRequesterPaths;
use crate::config::Config;
use crate::config::{default_config_filepath, Debug, NetworkRequester};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::Config as BaseClientConfig;
use nym_config::read_config_from_toml_file;
use nym_config::serde_helpers::de_maybe_stringified;
use nym_network_defaults::mainnet;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::time::Duration;
use url::Url;

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV5 {
    // *sigh* currently we point to the most recent config because that's the one that's 'correct'
    // but the moment we make breaking changes there, we'll have to update this config too.
    // I think we should always keep versioned base config, i.e. `ConfigV1`, `ConfigV2`, etc,
    // and then just make type alias for the current one, i.e. `type Config = ConfigV2`.
    // then in 'old' configs we could simply use the underlying type as opposed to the alias for easier upgrades.
    #[serde(flatten)]
    pub base: BaseClientConfig,

    #[serde(default)]
    pub network_requester: NetworkRequesterV5,

    pub storage_paths: NetworkRequesterPathsV3,

    #[serde(default)]
    pub network_requester_debug: DebugV5,

    pub logging: LoggingSettings,
}

impl ConfigV5 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    #[allow(dead_code)]
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }
}

impl From<ConfigV5> for Config {
    fn from(value: ConfigV5) -> Self {
        Config {
            base: value.base,
            network_requester: value.network_requester.into(),
            storage_paths: NetworkRequesterPaths {
                common_paths: value.storage_paths.common_paths,
            },
            network_requester_debug: value.network_requester_debug.into(),
            logging: value.logging,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkRequesterV5 {
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
    pub use_deprecated_allow_list: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub upstream_exit_policy_url: Option<Url>,
}

impl Default for NetworkRequesterV5 {
    fn default() -> Self {
        NetworkRequesterV5 {
            open_proxy: false,
            enabled_statistics: false,
            statistics_recipient: None,
            disable_poisson_rate: true,
            use_deprecated_allow_list: true,
            upstream_exit_policy_url: Some(
                mainnet::EXIT_POLICY_URL
                    .parse()
                    .expect("invalid default exit policy URL"),
            ),
        }
    }
}

impl From<NetworkRequesterV5> for NetworkRequester {
    fn from(value: NetworkRequesterV5) -> Self {
        NetworkRequester {
            open_proxy: value.open_proxy,
            disable_poisson_rate: value.disable_poisson_rate,
            upstream_exit_policy_url: value.upstream_exit_policy_url,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugV5 {
    /// Defines how often the standard allow list should get updated
    /// Deprecated
    #[serde(with = "humantime_serde")]
    pub standard_list_update_interval: Duration,
}

impl From<DebugV5> for Debug {
    fn from(value: DebugV5) -> Self {
        Debug {
            standard_list_update_interval: value.standard_list_update_interval,
        }
    }
}

impl Default for DebugV5 {
    fn default() -> Self {
        DebugV5 {
            standard_list_update_interval: DEFAULT_STANDARD_LIST_UPDATE_INTERVAL,
        }
    }
}
