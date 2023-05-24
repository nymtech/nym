// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::template::config_template;
use nym_client_core::config::disk_persistence::CommonClientPathfinder;
use nym_client_core::config::ClientCoreConfigTrait;
use nym_config::defaults::DEFAULT_WEBSOCKET_LISTENING_PORT;
use nym_config::{
    must_get_home, NymConfig, OptionalSet, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME,
    DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub use nym_client_core::config::Config as BaseConfig;
pub use nym_client_core::config::MISSING_VALUE;
pub use nym_client_core::config::{DebugConfig, GatewayEndpointConfig};

pub mod old_config_v1_1_13;
mod template;

const DEFAULT_CLIENTS_DIR: &str = "clients";

/// Derive default path to client's config file.
/// It should get resolved to `$HOME/.nym/clients/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
        .join(DEFAULT_CONFIG_FILENAME)
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

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseConfig,

    pub socket: Socket,

    pub paths: CommonClientPathfinder,

    pub logging: Logging,
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Config {
            base: BaseConfig::new(id.as_ref()),
            paths: CommonClientPathfinder::new_default(default_data_directory(id.as_ref())),
            logging: Default::default(),
            socket: Default::default(),
        }
    }

    pub fn get_gateway_endpoint(&self) -> &GatewayEndpointConfig {
        self.base.get_gateway_endpoint()
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

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    // pub fn get_base(&self) -> &BaseConfig<Self> {
    //     &self.base
    // }
    //
    // pub fn get_base_mut(&mut self) -> &mut BaseConfig<Self> {
    //     &mut self.base
    // }

    pub fn get_debug_settings(&self) -> &DebugConfig {
        self.get_base().get_debug_config()
    }

    pub fn get_socket_type(&self) -> SocketType {
        self.socket.socket_type
    }

    pub fn get_listening_ip(&self) -> IpAddr {
        self.socket.host
    }

    pub fn get_listening_port(&self) -> u16 {
        self.socket.listening_port
    }

    // // poor man's 'builder' method
    // pub fn with_base<F, T>(mut self, f: F, val: T) -> Self
    // where
    //     F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
    // {
    //     self.base = f(self.base, val);
    //     self
    // }
    //
    // // helper methods to use `OptionalSet` trait. Those are defined due to very... ehm. 'specific' structure of this config
    // // (plz, lets refactor it)
    // pub fn with_optional_ext<F, T>(mut self, f: F, val: Option<T>) -> Self
    // where
    //     F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
    // {
    //     self.base = self.base.with_optional(f, val);
    //     self
    // }
    //
    // pub fn with_optional_env_ext<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    // where
    //     F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
    //     T: FromStr,
    //     <T as FromStr>::Err: Debug,
    // {
    //     self.base = self.base.with_optional_env(f, val, env_var);
    //     self
    // }
    //
    // pub fn with_optional_custom_env_ext<F, T, G>(
    //     mut self,
    //     f: F,
    //     val: Option<T>,
    //     env_var: &str,
    //     parser: G,
    // ) -> Self
    // where
    //     F: Fn(BaseConfig<Self>, T) -> BaseConfig<Self>,
    //     G: Fn(&str) -> T,
    // {
    //     self.base = self.base.with_optional_custom_env(f, val, env_var, parser);
    //     self
    // }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Logging {}

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
