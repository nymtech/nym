// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::helpers::ephemeral_gateway_config;
use crate::config::persistence::EntryGatewayPaths;
use crate::config::Config;
use crate::error::EntryGatewayError;
use nym_config::defaults::DEFAULT_CLIENT_LISTENING_PORT;
use nym_config::helpers::inaddr_any;
use nym_config::serde_helpers::de_maybe_port;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;

pub const DEFAULT_WS_PORT: u16 = DEFAULT_CLIENT_LISTENING_PORT;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayConfig {
    pub storage_paths: EntryGatewayPaths,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet
    /// or if it also accepts non-paying clients
    pub enforce_zk_nyms: bool,

    /// Socket address this node will use for binding its client websocket API.
    /// default: `0.0.0.0:9000`
    pub bind_address: SocketAddr,

    /// Custom announced port for listening for websocket client traffic.
    /// If unspecified, the value from the `bind_address` will be used instead
    /// default: None
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_ws_port: Option<u16>,

    /// If applicable, announced port for listening for secure websocket client traffic.
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_wss_port: Option<u16>,

    #[serde(default)]
    pub debug: Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debug {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
}

impl Debug {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
        }
    }
}

impl EntryGatewayConfig {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        EntryGatewayConfig {
            storage_paths: EntryGatewayPaths::new(data_dir),
            enforce_zk_nyms: false,
            bind_address: SocketAddr::new(inaddr_any(), DEFAULT_WS_PORT),
            announce_ws_port: None,
            announce_wss_port: None,
            debug: Default::default(),
        }
    }
}

// a temporary solution until all nodes are even more tightly integrated
pub fn ephemeral_entry_gateway_config(
    config: Config,
    mnemonic: &bip39::Mnemonic,
) -> Result<nym_gateway::config::Config, EntryGatewayError> {
    Ok(ephemeral_gateway_config(config, mnemonic)?)
}
