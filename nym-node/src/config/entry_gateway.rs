// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::helpers::ephemeral_gateway_config;
use crate::config::persistence::EntryGatewayPaths;
use crate::config::Config;
use crate::error::EntryGatewayError;
use nym_config::defaults::DEFAULT_CLIENT_LISTENING_PORT;
use nym_config::helpers::inaddr_any;
use nym_config::serde_helpers::de_maybe_port;
use nym_gateway::node::LocalAuthenticatorOpts;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use super::helpers::{base_client_config, EphemeralConfig};
use super::LocalWireguardOpts;

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
#[serde(default)]
pub struct Debug {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,

    pub zk_nym_tickets: ZkNymTicketHandlerDebug,
}

impl Debug {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            zk_nym_tickets: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ZkNymTicketHandlerDebug {
    /// Specifies the multiplier for revoking a malformed/double-spent ticket
    /// (if it has to go all the way to the nym-api for verification)
    /// e.g. if one ticket grants 100Mb and `revocation_bandwidth_penalty` is set to 1.5,
    /// the client will lose 150Mb
    pub revocation_bandwidth_penalty: f32,

    /// Specifies the interval for attempting to resolve any failed, pending operations,
    /// such as ticket verification or redemption.
    #[serde(with = "humantime_serde")]
    pub pending_poller: Duration,

    pub minimum_api_quorum: f32,

    /// Specifies the minimum number of tickets this gateway will attempt to redeem.
    pub minimum_redemption_tickets: usize,

    /// Specifies the maximum time between two subsequent tickets redemptions.
    /// That's required as nym-apis will purge all ticket information for tickets older than 30 days.
    #[serde(with = "humantime_serde")]
    pub maximum_time_between_redemption: Duration,
}

impl ZkNymTicketHandlerDebug {
    pub const DEFAULT_REVOCATION_BANDWIDTH_PENALTY: f32 = 10.0;
    pub const DEFAULT_PENDING_POLLER: Duration = Duration::from_secs(300);
    pub const DEFAULT_MINIMUM_API_QUORUM: f32 = 0.8;
    pub const DEFAULT_MINIMUM_REDEMPTION_TICKETS: usize = 100;
    pub const DEFAULT_MAXIMUM_TIME_BETWEEN_REDEMPTION: Duration = Duration::from_secs(86400 * 25);
}

impl Default for ZkNymTicketHandlerDebug {
    fn default() -> Self {
        ZkNymTicketHandlerDebug {
            revocation_bandwidth_penalty: Self::DEFAULT_REVOCATION_BANDWIDTH_PENALTY,
            pending_poller: Self::DEFAULT_PENDING_POLLER,
            minimum_api_quorum: Self::DEFAULT_MINIMUM_API_QUORUM,
            minimum_redemption_tickets: Self::DEFAULT_MINIMUM_REDEMPTION_TICKETS,
            maximum_time_between_redemption: Self::DEFAULT_MAXIMUM_TIME_BETWEEN_REDEMPTION,
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
) -> Result<EphemeralConfig, EntryGatewayError> {
    let auth_opts = LocalAuthenticatorOpts {
        config: nym_authenticator::Config {
            base: nym_client_core_config_types::Config {
                client: base_client_config(&config),
                debug: config.authenticator.debug.client_debug,
            },
            authenticator: config.wireguard.clone().into(),
            storage_paths: nym_authenticator::config::AuthenticatorPaths {
                common_paths: config
                    .exit_gateway
                    .storage_paths
                    .authenticator
                    .to_common_client_paths(),
            },
            logging: config.logging,
        },
        custom_mixnet_path: None,
    };

    let wg_opts = LocalWireguardOpts {
        config: super::Wireguard {
            enabled: config.wireguard.enabled,
            bind_address: config.wireguard.bind_address,
            private_ip: config.wireguard.private_ip,
            announced_port: config.wireguard.announced_port,
            private_network_prefix: config.wireguard.private_network_prefix,
            storage_paths: config.wireguard.storage_paths.clone(),
        },
        custom_mixnet_path: None,
    };

    let gateway = ephemeral_gateway_config(config, mnemonic)?;
    Ok(EphemeralConfig {
        nr_opts: None,
        ipr_opts: None,
        auth_opts,
        wg_opts,
        gateway,
    })
}
