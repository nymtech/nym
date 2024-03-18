// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::EntryGatewayPaths;
use crate::config::Config;
use crate::error::EntryGatewayError;
use clap::crate_version;
use nym_config::defaults::DEFAULT_CLIENT_LISTENING_PORT;
use nym_config::helpers::inaddr_any;
use nym_config::serde_helpers::de_maybe_port;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::Path;
use zeroize::Zeroizing;

pub const DEFAULT_WS_PORT: u16 = DEFAULT_CLIENT_LISTENING_PORT;

#[derive(Debug, Serialize, Deserialize)]
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
    pub announce_ws_port: Option<u16>,

    /// If applicable, announced port for listening for secure websocket client traffic.
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_wss_port: Option<u16>,

    #[serde(default)]
    pub debug: Debug,
}

#[derive(Debug, Serialize, Deserialize)]
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
    mnemonic: Zeroizing<bip39::Mnemonic>,
) -> Result<nym_gateway::config::Config, EntryGatewayError> {
    let host = nym_gateway::config::Host {
        public_ips: config.host.public_ips,
        hostname: config.host.hostname,
    };

    let http = nym_gateway::config::Http {
        bind_address: config.http.bind_address,
        landing_page_assets_path: config.http.landing_page_assets_path,
    };

    let clients_bind_ip = config.entry_gateway.bind_address.ip();
    let mix_bind_ip = config.mixnet.bind_address.ip();
    if clients_bind_ip != mix_bind_ip {
        return Err(EntryGatewayError::UnsupportedAddresses {
            clients_bind_ip,
            mix_bind_ip,
        });
    }

    // SAFETY: we're using hardcoded valid url here (that won't be used anyway)
    #[allow(clippy::unwrap_used)]
    let gateway = nym_gateway::config::Gateway {
        // that field is very much irrelevant, but I guess let's keep them for now
        version: format!("{}-nym-node", crate_version!()),
        id: config.id,
        only_coconut_credentials: config.entry_gateway.enforce_zk_nyms,
        listening_address: clients_bind_ip,
        mix_port: config.mixnet.bind_address.port(),
        clients_port: config.entry_gateway.bind_address.port(),
        clients_wss_port: config.entry_gateway.announce_wss_port,
        enabled_statistics: false,
        statistics_service_url: "https://nymtech.net/foobar".parse().unwrap(),
        nym_api_urls: config.mixnet.nym_api_urls,
        nyxd_urls: config.mixnet.nyxd_urls,

        // that's nasty but can't do anything about it for this temporary solution : (
        cosmos_mnemonic: mnemonic.deref().clone(),
    };

    let wireguard = nym_gateway::config::Wireguard {
        enabled: config.wireguard.enabled,
        bind_address: config.wireguard.bind_address,
        announced_port: config.wireguard.announced_port,
        private_network_prefix: config.wireguard.private_network_prefix,
        storage_paths: nym_gateway::config::WireguardPaths::new_empty(),
    };

    Ok(nym_gateway::config::Config::externally_loaded(
        host,
        http,
        gateway,
        wireguard,
        nym_gateway::config::GatewayPaths::new_empty(),
        nym_gateway::config::NetworkRequester { enabled: false },
        nym_gateway::config::IpPacketRouter { enabled: false },
        config.logging,
        nym_gateway::config::Debug {
            packet_forwarding_initial_backoff: config
                .mixnet
                .debug
                .packet_forwarding_initial_backoff,
            packet_forwarding_maximum_backoff: config
                .mixnet
                .debug
                .packet_forwarding_maximum_backoff,
            initial_connection_timeout: config.mixnet.debug.initial_connection_timeout,
            maximum_connection_buffer_size: config.mixnet.debug.maximum_connection_buffer_size,
            message_retrieval_limit: config.entry_gateway.debug.message_retrieval_limit,
            use_legacy_framed_packet_version: false,
            ..Default::default()
        },
    ))
}
