// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::config::{
        default_config_filepath, persistence::ClientPaths, Config, Socket, SocketType,
    },
    error::ClientError,
};

use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_20_2::CommonClientPathsV1_1_20_2;
use nym_client_core::config::old_config_v1_1_20_2::ConfigV1_1_20_2 as BaseConfigV1_1_20_2;
use nym_client_core::config::old_config_v1_1_30::ConfigV1_1_30 as BaseConfigV1_1_30;
use nym_client_core::config::GatewayEndpointConfig;
use nym_config::read_config_from_toml_file;
use nym_network_defaults::DEFAULT_WEBSOCKET_LISTENING_PORT;
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct ClientPathsV1_1_20_2 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV1_1_20_2,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ConfigV1_1_20_2 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_20_2,

    pub socket: SocketV1_1_20_2,

    pub storage_paths: ClientPathsV1_1_20_2,

    pub logging: LoggingSettings,
}

impl ConfigV1_1_20_2 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    // in this upgrade, gateway endpoint configuration was moved out of the config file,
    // so its returned to be stored elsewhere.
    pub fn upgrade(self) -> Result<(Config, GatewayEndpointConfig), ClientError> {
        let gateway_details = self.base.client.gateway_endpoint.clone().into();
        let config = Config {
            base: BaseConfigV1_1_30::from(self.base).into(),
            socket: self.socket.into(),
            storage_paths: ClientPaths {
                common_paths: self.storage_paths.common_paths.upgrade_default()?,
            },
            logging: self.logging,
        };

        Ok((config, gateway_details))
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub enum SocketTypeV1_1_20_2 {
    WebSocket,
    None,
}

impl From<SocketTypeV1_1_20_2> for SocketType {
    fn from(value: SocketTypeV1_1_20_2) -> Self {
        match value {
            SocketTypeV1_1_20_2::WebSocket => SocketType::WebSocket,
            SocketTypeV1_1_20_2::None => SocketType::None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SocketV1_1_20_2 {
    pub socket_type: SocketTypeV1_1_20_2,
    pub host: IpAddr,
    pub listening_port: u16,
}

impl From<SocketV1_1_20_2> for Socket {
    fn from(value: SocketV1_1_20_2) -> Self {
        Socket {
            socket_type: value.socket_type.into(),
            host: value.host,
            listening_port: value.listening_port,
        }
    }
}

impl Default for SocketV1_1_20_2 {
    fn default() -> Self {
        SocketV1_1_20_2 {
            socket_type: SocketTypeV1_1_20_2::WebSocket,
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            listening_port: DEFAULT_WEBSOCKET_LISTENING_PORT,
        }
    }
}
