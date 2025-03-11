// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::GatewayTasksPaths;
use nym_config::defaults::{DEFAULT_CLIENT_LISTENING_PORT, TICKETBOOK_VALIDITY_DAYS};
use nym_config::helpers::in6addr_any_init;
use nym_config::serde_helpers::de_maybe_port;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

pub const DEFAULT_WS_PORT: u16 = DEFAULT_CLIENT_LISTENING_PORT;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksConfig {
    pub storage_paths: GatewayTasksPaths,

    /// Indicates whether this gateway is accepting only zk-nym credentials for accessing the mixnet
    /// or if it also accepts non-paying clients
    pub enforce_zk_nyms: bool,

    /// Socket address this node will use for binding its client websocket API.
    /// default: `[::]:9000`
    pub ws_bind_address: SocketAddr,

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

    /// The maximum number of client connections the gateway will keep open at once.
    pub maximum_open_connections: usize,

    /// Defines the maximum age of a signed authentication request before it's deemed too stale to process.
    pub maximum_auth_request_age: Duration,

    pub stale_messages: StaleMessageDebug,

    pub client_bandwidth: ClientBandwidthDebug,

    pub zk_nym_tickets: ZkNymTicketHandlerDebug,
}

impl Debug {
    pub const DEFAULT_MAXIMUM_OPEN_CONNECTIONS: usize = 8192;
    pub const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
    pub const DEFAULT_MAXIMUM_AUTH_REQUEST_AGE: Duration = Duration::from_secs(30);
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            maximum_open_connections: Self::DEFAULT_MAXIMUM_OPEN_CONNECTIONS,
            maximum_auth_request_age: Self::DEFAULT_MAXIMUM_AUTH_REQUEST_AGE,
            stale_messages: Default::default(),
            client_bandwidth: Default::default(),
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
    /// That's required as nym-apis will purge all ticket information for tickets older than maximum validity.
    #[serde(with = "humantime_serde")]
    pub maximum_time_between_redemption: Duration,
}

impl ZkNymTicketHandlerDebug {
    pub const DEFAULT_REVOCATION_BANDWIDTH_PENALTY: f32 = 10.0;
    pub const DEFAULT_PENDING_POLLER: Duration = Duration::from_secs(300);
    pub const DEFAULT_MINIMUM_API_QUORUM: f32 = 0.7;
    pub const DEFAULT_MINIMUM_REDEMPTION_TICKETS: usize = 100;

    // use min(4/5 of max validity, validity - 1), but making sure it's no greater than 1 day
    // ASSUMPTION: our validity period is AT LEAST 2 days
    //
    // this could have been a constant, but it's more readable as a function
    pub const fn default_maximum_time_between_redemption() -> Duration {
        let desired_secs = TICKETBOOK_VALIDITY_DAYS * (86400 * 4) / 5;
        let desired_secs_alt = (TICKETBOOK_VALIDITY_DAYS - 1) * 86400;

        // can't use `min` in const context
        let target_secs = if desired_secs < desired_secs_alt {
            desired_secs
        } else {
            desired_secs_alt
        };

        assert!(
            target_secs > 86400,
            "the maximum time between redemption can't be lower than 1 day!"
        );
        Duration::from_secs(target_secs as u64)
    }
}

impl Default for ZkNymTicketHandlerDebug {
    fn default() -> Self {
        ZkNymTicketHandlerDebug {
            revocation_bandwidth_penalty: Self::DEFAULT_REVOCATION_BANDWIDTH_PENALTY,
            pending_poller: Self::DEFAULT_PENDING_POLLER,
            minimum_api_quorum: Self::DEFAULT_MINIMUM_API_QUORUM,
            minimum_redemption_tickets: Self::DEFAULT_MINIMUM_REDEMPTION_TICKETS,
            maximum_time_between_redemption: Self::default_maximum_time_between_redemption(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClientBandwidthDebug {
    /// Defines maximum delay between client bandwidth information being flushed to the persistent storage.
    pub max_flushing_rate: Duration,

    /// Defines a maximum change in client bandwidth before it gets flushed to the persistent storage.
    pub max_delta_flushing_amount: i64,
}

impl ClientBandwidthDebug {
    const DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE: Duration = Duration::from_millis(5);
    const DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT: i64 = 512 * 1024; // 512kB
}

impl Default for ClientBandwidthDebug {
    fn default() -> Self {
        ClientBandwidthDebug {
            max_flushing_rate: Self::DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE,
            max_delta_flushing_amount: Self::DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StaleMessageDebug {
    /// Specifies how often the clean-up task should check for stale data.
    #[serde(with = "humantime_serde")]
    pub cleaner_run_interval: Duration,

    /// Specifies maximum age of stored messages before they are removed from the storage
    #[serde(with = "humantime_serde")]
    pub max_age: Duration,
}

impl StaleMessageDebug {
    const DEFAULT_STALE_MESSAGES_CLEANER_RUN_INTERVAL: Duration = Duration::from_secs(60 * 60);
    const DEFAULT_STALE_MESSAGES_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
}

impl Default for StaleMessageDebug {
    fn default() -> Self {
        StaleMessageDebug {
            cleaner_run_interval: Self::DEFAULT_STALE_MESSAGES_CLEANER_RUN_INTERVAL,
            max_age: Self::DEFAULT_STALE_MESSAGES_MAX_AGE,
        }
    }
}

impl GatewayTasksConfig {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        GatewayTasksConfig {
            storage_paths: GatewayTasksPaths::new(data_dir),
            enforce_zk_nyms: false,
            ws_bind_address: SocketAddr::new(in6addr_any_init(), DEFAULT_WS_PORT),
            announce_ws_port: None,
            announce_wss_port: None,
            debug: Default::default(),
        }
    }
}
