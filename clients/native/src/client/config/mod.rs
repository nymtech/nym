// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::persistence::ClientPaths;
use crate::client::config::template::CONFIG_TEMPLATE;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::cli_helpers::client_init::ClientConfig;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_config::defaults::DEFAULT_WEBSOCKET_LISTENING_PORT;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    OptionalSet, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub use nym_client_core::config::Config as BaseClientConfig;
pub use nym_client_core::config::{DebugConfig, GatewayEndpointConfig};

pub mod old_config_v1_1_13;
pub mod old_config_v1_1_20;
pub mod old_config_v1_1_20_2;
pub mod old_config_v1_1_33;
mod persistence;
mod template;

const DEFAULT_CLIENTS_DIR: &str = "clients";

/// Derive default path to clients's config directory.
/// It should get resolved to `$HOME/.nym/mixnodes/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to client's config file.
/// It should get resolved to `$HOME/.nym/clients/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to client's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/clients/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    pub socket: Socket,

    // pub paths: CommonClientPathfinder,
    pub storage_paths: ClientPaths,

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
            storage_paths: ClientPaths::new_default(default_data_directory(id.as_ref())),
            logging: Default::default(),
            socket: Default::default(),
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

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }

    pub fn with_socket(mut self, socket_type: SocketType) -> Self {
        self.socket.socket_type = socket_type;
        self
    }

    pub fn with_disabled_socket(mut self, disabled: bool) -> Self {
        if disabled {
            self.socket.socket_type = SocketType::None;
        } else {
            self.socket.socket_type = SocketType::WebSocket;
        }
        self
    }

    pub fn with_host(mut self, host: IpAddr) -> Self {
        self.socket.host = host;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.socket.listening_port = port;
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
    pub fn with_optional_ext<F, T>(mut self, f: F, val: Option<T>) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
    {
        self.base = self.base.with_optional(f, val);
        self
    }

    pub fn with_optional_env_ext<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
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
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
        G: Fn(&str) -> T,
    {
        self.base = self.base.with_optional_custom_env(f, val, env_var, parser);
        self
    }
}

// define_optional_set_inner!(Config, base, BaseClientConfig);

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub enum SocketType {
    WebSocket,
    None,
}

impl SocketType {
    pub fn is_websocket(&self) -> bool {
        matches!(self, SocketType::WebSocket)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Socket {
    pub socket_type: SocketType,
    pub host: IpAddr,
    pub listening_port: u16,
}

impl Default for Socket {
    fn default() -> Self {
        Socket {
            socket_type: SocketType::WebSocket,
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            listening_port: DEFAULT_WEBSOCKET_LISTENING_PORT,
        }
    }
}
