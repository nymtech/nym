// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::NetworkRequesterPaths;
use crate::config::template::CONFIG_TEMPLATE;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::cli_helpers::client_init::ClientConfig;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file,
    serde_helpers::de_maybe_stringified, NymConfigTemplate, OptionalSet, DEFAULT_CONFIG_DIR,
    DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_network_defaults::mainnet;
use nym_service_providers_common::DEFAULT_SERVICE_PROVIDERS_DIR;
use nym_sphinx::params::PacketSize;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use url::Url;

pub use nym_client_core::config::Config as BaseClientConfig;

pub mod old_config_v1_1_13;
pub mod old_config_v1_1_20;
pub mod old_config_v1_1_20_2;
pub mod old_config_v1_1_33;
mod persistence;
mod template;

const DEFAULT_NETWORK_REQUESTERS_DIR: &str = "network-requester";

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Derive default path to network requester's config directory.
/// It should get resolved to `$HOME/.nym/service-providers/network-requester/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SERVICE_PROVIDERS_DIR)
        .join(DEFAULT_NETWORK_REQUESTERS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to network requester's config file.
/// It should get resolved to `$HOME/.nym/service-providers/network-requester/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to network requester's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/service-providers/network-requester/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SERVICE_PROVIDERS_DIR)
        .join(DEFAULT_NETWORK_REQUESTERS_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

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

    pub logging: LoggingSettings,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl ClientConfig for Config {
    fn common_paths(&self) -> &CommonClientPaths {
        &self.storage_paths.common_paths
    }

    fn core_config(&self) -> &BaseClientConfig {
        &self.base
    }

    fn default_store_location(&self) -> PathBuf {
        self.default_location()
    }

    fn save_to<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        save_formatted_config_to_file(self, path)
    }
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Config {
            base: BaseClientConfig::new(id.as_ref(), env!("CARGO_PKG_VERSION")),
            network_requester: Default::default(),
            storage_paths: NetworkRequesterPaths::new_base(default_data_directory(id.as_ref())),
            network_requester_debug: Default::default(),
            logging: Default::default(),
        }
    }

    // this is a false positive, this method is actually called when used as a library
    // but clippy complains about it when building the binary
    #[allow(unused)]
    pub fn with_data_directory<P: AsRef<Path>>(mut self, data_directory: P) -> Self {
        self.storage_paths = NetworkRequesterPaths::new_base(data_directory);
        self
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.base.client.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[doc(hidden)]
    pub fn set_medium_toggle(&mut self) {
        self.base.set_no_cover_traffic_with_keepalive();
        self.base.set_no_per_hop_delays();
        self.base.debug.traffic.secondary_packet_size = Some(PacketSize::ExtendedPacket16);
    }

    #[doc(hidden)]
    pub fn set_no_poisson_process(&mut self) {
        self.base.set_no_poisson_process()
    }

    #[must_use]
    pub fn with_open_proxy(mut self, open_proxy: bool) -> Self {
        self.network_requester.open_proxy = open_proxy;
        self
    }

    #[must_use]
    pub fn with_old_allow_list(mut self, use_old_allow_list: bool) -> Self {
        self.network_requester.use_deprecated_allow_list = use_old_allow_list;
        self
    }

    #[must_use]
    pub fn with_enabled_statistics(mut self, enabled_statistics: bool) -> Self {
        self.network_requester.enabled_statistics = enabled_statistics;
        self
    }

    #[must_use]
    pub fn with_statistics_recipient(mut self, statistics_recipient: String) -> Self {
        self.network_requester.statistics_recipient = Some(statistics_recipient);
        self
    }

    // poor man's 'builder' method
    pub fn with_base<F, T>(mut self, f: F, val: T) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
    {
        self.base = f(self.base, val);
        self
    }

    // helper methods to use `OptionalSet` trait. Those are defined due to very... ehm. 'specific' structure of this config
    // (plz, lets refactor it)
    pub fn with_optional_base<F, T>(mut self, f: F, val: Option<T>) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
    {
        self.base = self.base.with_optional(f, val);
        self
    }

    #[allow(unused)]
    pub fn with_optional_base_env<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
    {
        self.base = self.base.with_optional_env(f, val, env_var);
        self
    }

    #[allow(unused)]
    pub fn with_optional_base_custom_env<F, T, G>(
        mut self,
        f: F,
        val: Option<T>,
        env_var: &str,
        parser: G,
    ) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
        G: Fn(&str) -> T,
    {
        self.base = self.base.with_optional_custom_env(f, val, env_var, parser);
        self
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkRequester {
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

impl Default for NetworkRequester {
    fn default() -> Self {
        NetworkRequester {
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
