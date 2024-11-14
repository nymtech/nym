// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_defaults::{DEFAULT_NYM_NODE_HTTP_PORT, TICKETBOOK_VALIDITY_DAYS};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

// 'DEBUG'
// where applicable, the below are defined in milliseconds
const DEFAULT_PRESENCE_SENDING_DELAY: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;

const DEFAULT_STORED_MESSAGE_FILENAME_LENGTH: u16 = 16;
const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;

const DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE: Duration = Duration::from_millis(5);
const DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT: i64 = 512 * 1024; // 512kB

#[derive(Debug)]
pub struct Config {
    pub host: Host,

    pub http: Http,

    pub gateway: Gateway,

    // pub storage_paths: GatewayPaths,
    pub network_requester: NetworkRequester,

    pub ip_packet_router: IpPacketRouter,

    pub debug: Debug,
}

impl Config {
    #[allow(clippy::too_many_arguments)]
    pub fn externally_loaded(
        host: impl Into<Host>,
        http: impl Into<Http>,
        gateway: impl Into<Gateway>,
        network_requester: impl Into<NetworkRequester>,
        ip_packet_router: impl Into<IpPacketRouter>,
        debug: impl Into<Debug>,
    ) -> Self {
        Config {
            host: host.into(),
            http: http.into(),
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

    pub fn get_cosmos_mnemonic(&self) -> bip39::Mnemonic {
        self.gateway.cosmos_mnemonic.clone()
    }
}

// TODO: this is very much a WIP. we need proper ssl certificate support here
#[derive(Debug, PartialEq)]
pub struct Host {
    /// Ip address(es) of this host, such as 1.1.1.1 that external clients will use for connections.
    pub public_ips: Vec<IpAddr>,

    /// Optional hostname of this node, for example nymtech.net.
    // TODO: this is temporary. to be replaced by pulling the data directly from the certs.
    pub hostname: Option<String>,
}

impl Host {
    pub fn validate(&self) -> bool {
        if self.public_ips.is_empty() {
            return false;
        }

        true
    }
}

#[derive(Debug, PartialEq)]
pub struct Http {
    /// Socket address this node will use for binding its http API.
    /// default: `0.0.0.0:8000`
    pub bind_address: SocketAddr,

    /// Path to assets directory of custom landing page of this node.
    pub landing_page_assets_path: Option<PathBuf>,
}

impl Default for Http {
    fn default() -> Self {
        Http {
            bind_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                DEFAULT_NYM_NODE_HTTP_PORT,
            ),
            landing_page_assets_path: None,
        }
    }
}

// we only really care about the mnemonic being zeroized
#[derive(Debug, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Gateway {
    /// Version of the gateway for which this configuration was created.
    pub version: String,

    /// ID specifies the human readable ID of this particular gateway.
    pub id: String,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the
    /// the mixnet, or if it also accepts non-paying clients
    pub only_coconut_credentials: bool,

    /// Address to which this mixnode will bind to and will be listening for packets.
    #[zeroize(skip)]
    pub listening_address: IpAddr,

    /// Port used for listening for all mixnet traffic.
    /// (default: 1789)
    pub mix_port: u16,

    /// Port used for listening for all client-related traffic.
    /// (default: 9000)
    pub clients_port: u16,

    /// If applicable, announced port for listening for secure websocket client traffic.
    /// (default: None)
    pub clients_wss_port: Option<u16>,

    /// Addresses to APIs from which the node gets the view of the network.
    #[zeroize(skip)]
    pub nym_api_urls: Vec<Url>,

    /// Addresses to validators which the node uses to check for double spending of ERC20 tokens.
    #[zeroize(skip)]
    pub nyxd_urls: Vec<Url>,

    /// Mnemonic of a cosmos wallet used in checking for double spending.
    // #[deprecated(note = "move to storage")]
    // TODO: I don't think this should be stored directly in the config...
    pub cosmos_mnemonic: bip39::Mnemonic,
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
    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    pub packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    pub packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    pub initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    pub maximum_connection_buffer_size: usize,

    /// Delay between each subsequent presence data being sent.
    // DEAD FIELD
    pub presence_sending_delay: Duration,

    /// Length of filenames for new client messages.
    // DEAD FIELD
    pub stored_messages_filename_length: u16,

    /// Number of messages from offline client that can be pulled at once from the storage.
    pub message_retrieval_limit: i64,

    /// Defines maximum delay between client bandwidth information being flushed to the persistent storage.
    pub client_bandwidth_max_flushing_rate: Duration,

    /// Defines a maximum change in client bandwidth before it gets flushed to the persistent storage.
    pub client_bandwidth_max_delta_flushing_amount: i64,

    /// Specifies whether the mixnode should be using the legacy framing for the sphinx packets.
    // it's set to true by default. The reason for that decision is to preserve compatibility with the
    // existing nodes whilst everyone else is upgrading and getting the code for handling the new field.
    // It shall be disabled in the subsequent releases.
    pub use_legacy_framed_packet_version: bool,

    pub zk_nym_tickets: ZkNymTicketHandlerDebug,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            presence_sending_delay: DEFAULT_PRESENCE_SENDING_DELAY,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            stored_messages_filename_length: DEFAULT_STORED_MESSAGE_FILENAME_LENGTH,
            message_retrieval_limit: DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            client_bandwidth_max_flushing_rate: DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE,
            client_bandwidth_max_delta_flushing_amount:
                DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT,
            use_legacy_framed_packet_version: false,
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
