// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bin_common::logging::LoggingSettings;
pub use nym_client_core::config::Config as BaseClientConfig;
use nym_client_core::{cli_helpers::CliClientConfig, config::disk_persistence::CommonClientPaths};
use nym_config::{
    must_get_home, save_formatted_config_to_file, NymConfigTemplate, OptionalSet,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_network_defaults::{
    WG_PORT, WG_TUN_DEVICE_IP_ADDRESS_V4, WG_TUN_DEVICE_IP_ADDRESS_V6, WG_TUN_DEVICE_NETMASK_V4,
    WG_TUN_DEVICE_NETMASK_V6,
};
use nym_service_providers_common::DEFAULT_SERVICE_PROVIDERS_DIR;
pub use persistence::AuthenticatorPaths;
use serde::{Deserialize, Serialize};
use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};
use template::CONFIG_TEMPLATE;

pub mod helpers;
pub mod persistence;
pub mod template;

const DEFAULT_AUTHENTICATOR_DIR: &str = "authenticator";

/// Derive default path to authenticator's config directory.
/// It should get resolved to `$HOME/.nym/service-providers/authenticator/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SERVICE_PROVIDERS_DIR)
        .join(DEFAULT_AUTHENTICATOR_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to authenticator's config file.
/// It should get resolved to `$HOME/.nym/service-providers/authenticator/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to authenticator's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/service-providers/authenticator/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SERVICE_PROVIDERS_DIR)
        .join(DEFAULT_AUTHENTICATOR_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    #[serde(default)]
    pub authenticator: Authenticator,

    pub storage_paths: AuthenticatorPaths,

    pub logging: LoggingSettings,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl CliClientConfig for Config {
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
            authenticator: Default::default(),
            storage_paths: AuthenticatorPaths::new_base(default_data_directory(id.as_ref())),
            logging: Default::default(),
        }
    }

    #[allow(unused)]
    pub fn with_data_directory<P: AsRef<Path>>(mut self, data_directory: P) -> Self {
        self.storage_paths = AuthenticatorPaths::new_base(data_directory);
        self
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        nym_config::read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.base.client.id)
    }

    #[allow(unused)]
    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }

    #[doc(hidden)]
    pub fn set_no_poisson_process(&mut self) {
        self.base.set_no_poisson_process()
    }

    // poor man's 'builder' method
    #[allow(unused)]
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
pub struct Authenticator {
    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    pub bind_address: SocketAddr,

    /// Private IP address of the wireguard gateway.
    /// default: `10.1.0.1`
    pub private_ipv4: Ipv4Addr,

    /// Private IP address of the wireguard gateway.
    /// default: `fc01::1`
    pub private_ipv6: Ipv6Addr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv4.
    /// The maximum value for IPv4 is 32
    pub private_network_prefix_v4: u8,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv6.
    /// The maximum value for IPv6 is 128
    pub private_network_prefix_v6: u8,
}

impl Default for Authenticator {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), WG_PORT),
            private_ipv4: WG_TUN_DEVICE_IP_ADDRESS_V4,
            private_ipv6: WG_TUN_DEVICE_IP_ADDRESS_V6,
            announced_port: WG_PORT,
            private_network_prefix_v4: WG_TUN_DEVICE_NETMASK_V4,
            private_network_prefix_v6: WG_TUN_DEVICE_NETMASK_V6,
        }
    }
}

impl From<Authenticator> for nym_wireguard_types::Config {
    fn from(value: Authenticator) -> Self {
        nym_wireguard_types::Config {
            bind_address: value.bind_address,
            private_ipv4: value.private_ipv4,
            private_ipv6: value.private_ipv6,
            announced_port: value.announced_port,
            private_network_prefix_v4: value.private_network_prefix_v4,
            private_network_prefix_v6: value.private_network_prefix_v6,
        }
    }
}
