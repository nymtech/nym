pub use nym_client_core::config::Config as BaseClientConfig;

use nym_bin_common::logging::LoggingSettings;
use nym_client_core::{
    cli_helpers::client_init::ClientConfig, config::disk_persistence::CommonClientPaths,
};
use nym_config::{
    defaults::mainnet, must_get_home, save_formatted_config_to_file,
    serde_helpers::de_maybe_stringified, NymConfigTemplate, OptionalSet, DEFAULT_CONFIG_DIR,
    DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_service_providers_common::DEFAULT_SERVICE_PROVIDERS_DIR;
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

use crate::config::persistence::IpPacketRouterPaths;

use self::template::CONFIG_TEMPLATE;

mod persistence;
mod template;

const DEFAULT_IP_PACKET_ROUTER_DIR: &str = "ip-packet-router";

/// Derive default path to ip packet routers' config directory.
/// It should get resolved to `$HOME/.nym/service-providers/ip-packet-router/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SERVICE_PROVIDERS_DIR)
        .join(DEFAULT_IP_PACKET_ROUTER_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to ip packet routers' config file.
/// It should get resolved to `$HOME/.nym/service-providers/ip-packet-router/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to network requester's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/service-providers/network-requester/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SERVICE_PROVIDERS_DIR)
        .join(DEFAULT_IP_PACKET_ROUTER_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    #[serde(default)]
    pub ip_packet_router: IpPacketRouter,

    pub storage_paths: IpPacketRouterPaths,

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
            ip_packet_router: Default::default(),
            storage_paths: IpPacketRouterPaths::new_base(default_data_directory(id.as_ref())),
            logging: Default::default(),
        }
    }

    #[allow(unused)]
    pub fn with_data_directory<P: AsRef<Path>>(mut self, data_directory: P) -> Self {
        self.storage_paths = IpPacketRouterPaths::new_base(data_directory);
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
pub struct IpPacketRouter {
    /// Disable Poisson sending rate.
    pub disable_poisson_rate: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub upstream_exit_policy_url: Option<Url>,
}

impl Default for IpPacketRouter {
    fn default() -> Self {
        IpPacketRouter {
            disable_poisson_rate: true,
            upstream_exit_policy_url: Some(
                mainnet::EXIT_POLICY_URL
                    .parse()
                    .expect("invalid default exit policy URL"),
            ),
        }
    }
}
