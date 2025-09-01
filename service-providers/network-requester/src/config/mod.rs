// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_config::serde_helpers::de_maybe_stringified;
use nym_network_defaults::mainnet;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use url::Url;

pub use crate::config::persistence::NetworkRequesterPaths;
pub use nym_client_core::config::Config as BaseClientConfig;

mod persistence;

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    #[serde(default)]
    pub network_requester: NetworkRequester,

    pub storage_paths: NetworkRequesterPaths,

    #[serde(default)]
    pub network_requester_debug: Debug,
}

impl Config {
    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[must_use]
    pub fn with_data_directory<P: AsRef<Path>>(mut self, data_directory: P) -> Self {
        self.storage_paths = NetworkRequesterPaths::new_base(data_directory);
        self
    }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }

    #[must_use]
    pub fn with_open_proxy(mut self, open_proxy: bool) -> Self {
        self.network_requester.open_proxy = open_proxy;
        self
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NetworkRequester {
    /// specifies whether this network requester should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting debug.traffic.disable_main_poisson_packet_distribution = true,
    pub disable_poisson_rate: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub upstream_exit_policy_url: Option<Url>,
}

impl Default for NetworkRequester {
    fn default() -> Self {
        NetworkRequester {
            open_proxy: false,
            disable_poisson_rate: true,
            upstream_exit_policy_url: Some(
                mainnet::EXIT_POLICY_URL
                    .parse()
                    .expect("invalid default exit policy URL"),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Debug {
    /// Defines how often the standard allow list should get updated
    /// Deprecated
    #[serde(with = "humantime_serde")]
    pub standard_list_update_interval: Duration,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            standard_list_update_interval: DEFAULT_STANDARD_LIST_UPDATE_INTERVAL,
        }
    }
}
