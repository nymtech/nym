// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_defaults::TICKETBOOK_VALIDITY_DAYS;
use std::net::SocketAddr;
use std::time::Duration;
use url::Url;

// TODO: can we move those away?
pub const DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE: Duration = Duration::from_millis(5);
pub const DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT: i64 = 512 * 1024; // 512kB

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

    pub zk_nym_tickets: ZkNymTicketHandlerDebug,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            client_bandwidth_max_flushing_rate: DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE,
            client_bandwidth_max_delta_flushing_amount:
                DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT,
            zk_nym_tickets: Default::default(),
        }
    }
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

impl ZkNymTicketHandlerDebug {
    pub const DEFAULT_REVOCATION_BANDWIDTH_PENALTY: f32 = 10.0;
    pub const DEFAULT_PENDING_POLLER: Duration = Duration::from_secs(300);
    pub const DEFAULT_MINIMUM_API_QUORUM: f32 = 0.8;
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
