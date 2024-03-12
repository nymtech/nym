// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::CONFIG_TEMPLATE;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::cli_helpers::CliClientConfig;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub use crate::config::persistence::SocksClientPaths;
pub use nym_client_core::config::Config as BaseClientConfig;
pub use nym_socks5_client_core::config::Config as CoreConfig;

pub mod old_config_v1_1_13;
pub mod old_config_v1_1_20;
pub mod old_config_v1_1_20_2;
pub mod old_config_v1_1_30;
pub mod old_config_v1_1_33;
mod persistence;
mod template;

const DEFAULT_SOCKS5_CLIENTS_DIR: &str = "socks5-clients";

/// Derive default path to clients's config directory.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SOCKS5_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to client's config file.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to client's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SOCKS5_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub core: CoreConfig,

    pub storage_paths: SocksClientPaths,

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
        &self.core.base
    }

    fn default_store_location(&self) -> PathBuf {
        self.default_location()
    }

    fn save_to<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        save_formatted_config_to_file(self, path)
    }
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S, provider_mix_address: S) -> Self {
        Config {
            core: CoreConfig::new(
                id.as_ref(),
                env!("CARGO_PKG_VERSION"),
                provider_mix_address.as_ref(),
            ),
            storage_paths: SocksClientPaths::new_default(default_data_directory(id.as_ref())),
            logging: Default::default(),
        }
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.core.base.client.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.core.validate()
    }

    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.core = self.core.with_port(port);
        self
    }

    #[must_use]
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.core = self.core.with_ip(ip);
        self
    }

    pub fn with_anonymous_replies(mut self, anonymous_replies: bool) -> Self {
        self.core.socks5.send_anonymously = anonymous_replies;
        self
    }

    // poor man's 'builder' method

    pub fn with_base<F, T>(mut self, f: F, val: T) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
    {
        self.core = self.core.with_base(f, val);
        self
    }

    pub fn with_optional_base<F, T>(mut self, f: F, val: Option<T>) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
    {
        self.core = self.core.with_optional_base(f, val);
        self
    }

    #[allow(unused)]
    pub fn with_optional_base_env<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
        T: FromStr,
        <T as FromStr>::Err: Debug,
    {
        self.core = self.core.with_optional_base_env(f, val, env_var);
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
        self.core = self
            .core
            .with_optional_base_custom_env(f, val, env_var, parser);
        self
    }
}
