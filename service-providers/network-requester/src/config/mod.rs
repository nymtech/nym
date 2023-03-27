// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::config_template;
use client_core::config::ClientCoreConfigTrait;
use nym_config::{NymConfig, OptionalSet};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;

pub use client_core::config::Config as BaseConfig;
pub use client_core::config::MISSING_VALUE;
pub use client_core::config::{DebugConfig, GatewayEndpointConfig};

pub mod old_config_v1_1_13;
mod template;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    base: BaseConfig<Config>,
}

impl NymConfig for Config {
    fn template() -> &'static str {
        config_template()
    }

    // TODO: merge base dir with `HostStore`.
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
    fn get_gateway_endpoint(&self) -> &client_core::config::GatewayEndpointConfig {
        self.base.get_gateway_endpoint()
    }
}

impl Config {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Config {
            base: BaseConfig::new(id),
        }
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
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
        F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
        G: Fn(&str) -> T,
    {
        self.base = self.base.with_optional_custom_env(f, val, env_var, parser);
        self
    }
}
