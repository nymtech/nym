// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::config_template;
use nym_client_core::config::ClientCoreConfigTrait;
use nym_config::{NymConfig, OptionalSet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

pub use nym_client_core::config::Config as BaseConfig;
pub use nym_client_core::config::MISSING_VALUE;
pub use nym_client_core::config::{DebugConfig, GatewayEndpointConfig};

pub const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

pub mod old_config_v1_1_13;
mod template;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    base: BaseConfig<Config>,

    #[serde(default)]
    pub network_requester: NetworkRequster,

    #[serde(default)]
    pub network_requester_debug: Debug,
}

impl NymConfig for Config {
    fn template() -> &'static str {
        config_template()
    }

    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("service-providers")
            .join("network-requester")
    }

    fn try_default_root_directory() -> Option<PathBuf> {
        dirs::home_dir().map(|path| path.join(".nym").join("clients"))
    }

    fn root_directory(&self) -> PathBuf {
        self.base.get_nym_root_directory()
    }

    fn config_directory(&self) -> PathBuf {
        self.root_directory()
            .join(self.base.get_id())
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.root_directory().join(self.base.get_id()).join("data")
    }
}

impl ClientCoreConfigTrait for Config {
    fn get_gateway_endpoint(&self) -> &nym_client_core::config::GatewayEndpointConfig {
        self.base.get_gateway_endpoint()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkRequster {
    /// Location of the file containing our allow.list
    pub allowed_list_location: PathBuf,

    /// Location of the file containing our unknown.list
    pub unknown_list_location: PathBuf,
}

impl Default for NetworkRequster {
    fn default() -> Self {
        // same defaults as we had in <= v1.1.13
        NetworkRequster {
            allowed_list_location: <Config as NymConfig>::default_root_directory()
                .join("allowed.list"),
            unknown_list_location: <Config as NymConfig>::default_root_directory()
                .join("unknown.list"),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Debug {
    /// Defines how often the standard allow list should get updated
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

impl Config {
    pub fn new<S: Into<String>>(id: S) -> Self {
        let mut cfg = Config {
            base: BaseConfig::new(id),
            ..Default::default()
        };

        cfg.network_requester.allowed_list_location = cfg.data_directory().join("allowed.list");
        cfg.network_requester.unknown_list_location = cfg.data_directory().join("unknown.list");
        cfg
    }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    pub fn allow_list_file_location(&self) -> PathBuf {
        self.network_requester.allowed_list_location.clone()
    }

    pub fn unknown_list_file_location(&self) -> PathBuf {
        self.network_requester.unknown_list_location.clone()
    }

    pub fn get_base(&self) -> &BaseConfig<Self> {
        &self.base
    }

    pub fn get_base_mut(&mut self) -> &mut BaseConfig<Self> {
        &mut self.base
    }

    // poor man's 'builder' method
    pub fn with_base<F, T>(mut self, f: F, val: T) -> Self
    where
        F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
    {
        self.base = f(self.base, val);
        self
    }

    // helper methods to use `OptionalSet` trait. Those are defined due to very... ehm. 'specific' structure of this config
    // (plz, lets refactor it)
    pub fn with_optional_ext<F, T>(mut self, f: F, val: Option<T>) -> Self
    where
        F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
    {
        self.base = self.base.with_optional(f, val);
        self
    }

    #[allow(dead_code)]
    pub fn with_optional_env_ext<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
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
        F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
        G: Fn(&str) -> T,
    {
        self.base = self.base.with_optional_custom_env(f, val, env_var, parser);
        self
    }
}
