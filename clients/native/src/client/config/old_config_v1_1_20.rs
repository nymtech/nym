// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::old_config_v1_1_20_2::{
    ClientPathsV1_1_20_2, ConfigV1_1_20_2, SocketTypeV1_1_20_2, SocketV1_1_20_2,
};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_20_2::CommonClientPathsV1_1_20_2;
use nym_client_core::config::disk_persistence::old_v1_1_33::ClientKeysPathsV1_1_33;
use nym_client_core::config::old_config_v1_1_20::ConfigV1_1_20 as BaseConfigV1_1_20;
use nym_client_core::config::old_config_v1_1_20_2::{
    ClientV1_1_20_2, ConfigV1_1_20_2 as BaseConfigV1_1_20_2,
};
use nym_config::defaults::DEFAULT_WEBSOCKET_LISTENING_PORT;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub enum SocketTypeV1_1_20 {
    WebSocket,
    None,
}

impl From<SocketTypeV1_1_20> for SocketTypeV1_1_20_2 {
    fn from(value: SocketTypeV1_1_20) -> Self {
        match value {
            SocketTypeV1_1_20::WebSocket => SocketTypeV1_1_20_2::WebSocket,
            SocketTypeV1_1_20::None => SocketTypeV1_1_20_2::None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_20 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_20<ConfigV1_1_20>,

    pub socket: SocketV1_1_20,
}

impl From<ConfigV1_1_20> for ConfigV1_1_20_2 {
    fn from(value: ConfigV1_1_20) -> Self {
        ConfigV1_1_20_2 {
            base: BaseConfigV1_1_20_2 {
                client: ClientV1_1_20_2 {
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
            storage_paths: ClientPathsV1_1_20_2 {
                common_paths: CommonClientPathsV1_1_20_2 {
                    keys: ClientKeysPathsV1_1_33 {
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

impl MigrationNymConfig for ConfigV1_1_20 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("clients")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SocketV1_1_20 {
    socket_type: SocketTypeV1_1_20,
    host: IpAddr,
    listening_port: u16,
}

impl From<SocketV1_1_20> for SocketV1_1_20_2 {
    fn from(value: SocketV1_1_20) -> Self {
        SocketV1_1_20_2 {
            socket_type: value.socket_type.into(),
            host: value.host,
            listening_port: value.listening_port,
        }
    }
}

impl Default for SocketV1_1_20 {
    fn default() -> Self {
        SocketV1_1_20 {
            socket_type: SocketTypeV1_1_20::WebSocket,
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listening_port: DEFAULT_WEBSOCKET_LISTENING_PORT,
        }
    }
}
