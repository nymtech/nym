// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::{config_template, CONFIG_TEMPLATE};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::CommonClientPathfinder;
pub use nym_client_core::config::Config as BaseConfig;
use nym_client_core::config::{ClientCoreConfigTrait, DebugConfig};
use nym_config::defaults::DEFAULT_SOCKS5_LISTENING_PORT;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfig,
    NymConfigTemplate, OptionalSet, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR,
    NYM_DIR,
};
use nym_service_providers_common::interface::ProviderInterfaceVersion;
pub use nym_socks5_client_core::config::Config as CoreConfig;
use nym_socks5_requests::Socks5ProtocolVersion;
use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod old_config_v1_1_13;
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

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub core: CoreConfig,

    pub paths: CommonClientPathfinder,

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
            core: CoreConfig::new(id.as_ref(), provider_mix_address),
            paths: CommonClientPathfinder::new_default(default_data_directory(id.as_ref())),
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
        default_config_filepath(&self.base.client.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    //
    // pub fn new<S: Into<String>>(id: S, provider_mix_address: S) -> Self {
    //     Config {
    //         base: BaseConfig::new(id),
    //         socks5: Socks5::new(provider_mix_address),
    //     }
    // }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.core.validate()
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.socks5.with_port(port);
        self
    }

    pub fn with_anonymous_replies(mut self, anonymous_replies: bool) -> Self {
        self.socks5.with_anonymous_replies(anonymous_replies);
        self
    }

    // poor man's 'builder' method
    pub fn with_core<F, T>(mut self, f: F, val: T) -> Self
    where
        F: Fn(CoreConfig, T) -> CoreConfig,
    {
        self.base = f(self.base, val);
        self
    }

    pub fn with_base<F, T>(mut self, f: F, val: T) -> Self
    where
        F: Fn(BaseConfig, T) -> BaseConfig,
    {
        self.core = self.core.with_base(f, val);
        self
    }

    // helper methods to use `OptionalSet` trait. Those are defined due to very... ehm. 'specific' structure of this config
    // (plz, lets refactor it)
    pub fn with_optional_ext<F, T>(mut self, f: F, val: Option<T>) -> Self
    where
        F: Fn(CoreConfig, T) -> CoreConfig,
    {
        self.base = self.base.with_optional(f, val);
        self
    }

    pub fn with_optional_env_ext<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(CoreConfig, T) -> CoreConfig,
        T: FromStr,
        <T as FromStr>::Err: Debug,
    {
        self.base = self.base.with_optional_env(f, val, env_var);
        self
    }

    pub fn with_optional_custom_env_ext<F, T, G>(
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
        self.base = self.base.with_optional_custom_env(f, val, env_var, parser);
        self
    }
}
