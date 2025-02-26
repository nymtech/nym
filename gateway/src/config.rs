// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::SocketAddr;
use std::time::Duration;
use url::Url;

#[derive(Debug)]
pub struct Config {
    pub gateway: Gateway,

    pub network_requester: NetworkRequester,

    pub ip_packet_router: IpPacketRouter,

    pub debug: Debug,
}

impl Config {
    pub fn new(
        gateway: impl Into<Gateway>,
        network_requester: impl Into<NetworkRequester>,
        ip_packet_router: impl Into<IpPacketRouter>,
        debug: impl Into<Debug>,
    ) -> Self {
        Config {
            gateway: gateway.into(),
            network_requester: network_requester.into(),
            ip_packet_router: ip_packet_router.into(),
            debug: debug.into(),
        }
    }

    pub fn get_nym_api_endpoints(&self) -> Vec<Url> {
        self.gateway.nym_api_urls.clone()
    }

    pub fn get_nyxd_urls(&self) -> Vec<Url> {
        self.gateway.nyxd_urls.clone()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Gateway {
    /// Indicates whether this gateway is accepting only zk-nym credentials for accessing the mixnet
    /// or if it also accepts non-paying clients
    pub enforce_zk_nyms: bool,

    /// Socket address this node will use for binding its client websocket API.
    /// default: `0.0.0.0:9000`
    pub websocket_bind_address: SocketAddr,

    /// Addresses to APIs from which the node gets the view of the network.
    pub nym_api_urls: Vec<Url>,

    /// Addresses to validators which the node uses to check for double spending of ERC20 tokens.
    pub nyxd_urls: Vec<Url>,
}

#[derive(Debug, PartialEq)]
pub struct NetworkRequester {
    /// Specifies whether network requester service is enabled in this process.
    pub enabled: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequester {
    fn default() -> Self {
        NetworkRequester { enabled: false }
    }
}

#[derive(Debug, PartialEq)]
pub struct IpPacketRouter {
    /// Specifies whether ip packet router service is enabled in this process.
    pub enabled: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for IpPacketRouter {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug)]
pub struct Debug {
    /// Defines maximum delay between client bandwidth information being flushed to the persistent storage.
    pub client_bandwidth_max_flushing_rate: Duration,

    /// Defines a maximum change in client bandwidth before it gets flushed to the persistent storage.
    pub client_bandwidth_max_delta_flushing_amount: i64,

    /// Specifies how often the clean-up task should check for stale data.
    pub stale_messages_cleaner_run_interval: Duration,

    /// Specifies maximum age of stored messages before they are removed from the storage
    pub stale_messages_max_age: Duration,

    /// The maximum number of client connections the gateway will keep open at once.
    pub maximum_open_connections: usize,

    pub zk_nym_tickets: ZkNymTicketHandlerDebug,
}

#[derive(Debug, Clone)]
pub struct ZkNymTicketHandlerDebug {
    /// Specifies the multiplier for revoking a malformed/double-spent ticket
    /// (if it has to go all the way to the nym-api for verification)
    /// e.g. if one ticket grants 100Mb and `revocation_bandwidth_penalty` is set to 1.5,
    /// the client will lose 150Mb
    pub revocation_bandwidth_penalty: f32,

    /// Specifies the interval for attempting to resolve any failed, pending operations,
    /// such as ticket verification or redemption.
    pub pending_poller: Duration,

    pub minimum_api_quorum: f32,

    /// Specifies the minimum number of tickets this gateway will attempt to redeem.
    pub minimum_redemption_tickets: usize,

    /// Specifies the maximum time between two subsequent tickets redemptions.
    /// That's required as nym-apis will purge all ticket information for tickets older than maximum validity.
    pub maximum_time_between_redemption: Duration,
}
