// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::SocksClientPaths;
use crate::config::template::CONFIG_TEMPLATE;
use nym_bin_common::logging::LoggingSettings;
pub use nym_client_core::config::Config as BaseClientConfig;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    OptionalSet, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
pub use nym_socks5_client_core::config::Config as CoreConfig;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod old_config_v1_1_13;
mod persistence;
mod template;

const DEFAULT_SOCKS5_CLIENTS_DIR: &str = "socks5-clients";

/// Derive default path to client's config file.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_SOCKS5_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
        .join(DEFAULT_CONFIG_FILENAME)
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
    #[serde(flatten)]
    pub core: CoreConfig,

    pub paths: SocksClientPaths,

    pub logging: LoggingSettings,
}

impl NymConfigTemplate for Config {
    fn template() -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S, provider_mix_address: S) -> Self {
        Config {
            core: CoreConfig::new(id.as_ref(), provider_mix_address.as_ref()),
            paths: SocksClientPaths::new_default(default_data_directory(id.as_ref())),
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

    //
    // pub fn new<S: Into<String>>(id: S, provider_mix_address: S) -> Self {
    //     Config {
    //         base: BaseClientConfig::new(id),
    //         socks5: Socks5::new(provider_mix_address),
    //     }
    // }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.core.validate()
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.core.socks5.listening_port = port;
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

    pub fn with_core<F, T>(mut self, f: F, val: T) -> Self
    where
        F: Fn(CoreConfig, T) -> CoreConfig,
    {
        self.core = f(self.core, val);
        self
    }

    pub fn with_optional_core<F, T>(mut self, f: F, val: Option<T>) -> Self
    where
        F: Fn(CoreConfig, T) -> CoreConfig,
    {
        self.core = self.core.with_optional(f, val);
        self
    }

    pub fn with_optional_core_env<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(CoreConfig, T) -> CoreConfig,
        T: FromStr,
        <T as FromStr>::Err: Debug,
    {
        self.core = self.core.with_optional_env(f, val, env_var);
        self
    }

    pub fn with_optional_core_custom_env<F, T, G>(
        mut self,
        f: F,
        val: Option<T>,
        env_var: &str,
        parser: G,
    ) -> Self
    where
        F: Fn(CoreConfig, T) -> CoreConfig,
        G: Fn(&str) -> T,
    {
        self.core = self.core.with_optional_custom_env(f, val, env_var, parser);
        self
    }
}
