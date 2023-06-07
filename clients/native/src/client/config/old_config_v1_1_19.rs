// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::persistence::ClientPaths;
use crate::client::config::{Config, Socket, SocketType};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::keys_paths::ClientKeysPaths;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_client_core::config::old_config_v1_1_19::ConfigV1_1_19 as BaseConfigV1_1_19;
use nym_client_core::config::{Client, Config as BaseConfig};
use nym_config::defaults::DEFAULT_WEBSOCKET_LISTENING_PORT;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub enum SocketTypeV1_1_19 {
    WebSocket,
    None,
}

impl From<SocketTypeV1_1_19> for SocketType {
    fn from(value: SocketTypeV1_1_19) -> Self {
        match value {
            SocketTypeV1_1_19::WebSocket => SocketType::WebSocket,
            SocketTypeV1_1_19::None => SocketType::None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_19 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_19<ConfigV1_1_19>,

    pub socket: SocketV1_1_19,
}

impl From<ConfigV1_1_19> for Config {
    fn from(value: ConfigV1_1_19) -> Self {
        Config {
            base: BaseConfig {
                client: Client {
                    version: value.base.client.version,
                    id: value.base.client.id,
                    disabled_credentials_mode: value.base.client.disabled_credentials_mode,
                    nyxd_urls: value.base.client.nyxd_urls,
                    nym_api_urls: value.base.client.nym_api_urls,
                    gateway_endpoint: value.base.client.gateway_endpoint.into(),
                },
                debug: value.base.debug.into(),
            },
            socket: value.socket.into(),
            storage_paths: ClientPaths {
                common_paths: CommonClientPaths {
                    keys: ClientKeysPaths {
                        private_identity_key_file: value.base.client.private_identity_key_file,
                        public_identity_key_file: value.base.client.public_identity_key_file,
                        private_encryption_key_file: value.base.client.private_encryption_key_file,
                        public_encryption_key_file: value.base.client.public_encryption_key_file,
                        gateway_shared_key_file: value.base.client.gateway_shared_key_file,
                        ack_key_file: value.base.client.ack_key_file,
                    },
                    credentials_database: value.base.client.database_path,
                    reply_surb_database: value.base.client.reply_surb_database_path,
                },
            },
            logging: LoggingSettings::default(),
        }
    }
}

impl MigrationNymConfig for ConfigV1_1_19 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("clients")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SocketV1_1_19 {
    socket_type: SocketTypeV1_1_19,
    host: IpAddr,
    listening_port: u16,
}

impl From<SocketV1_1_19> for Socket {
    fn from(value: SocketV1_1_19) -> Self {
        Socket {
            socket_type: value.socket_type.into(),
            host: value.host,
            listening_port: value.listening_port,
        }
    }
}

impl Default for SocketV1_1_19 {
    fn default() -> Self {
        SocketV1_1_19 {
            socket_type: SocketTypeV1_1_19::WebSocket,
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listening_port: DEFAULT_WEBSOCKET_LISTENING_PORT,
        }
    }
}
