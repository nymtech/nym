// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::authenticator::{Authenticator, AuthenticatorDebug};
use crate::config::gateway_tasks::{
    ClientBandwidthDebug, StaleMessageDebug, ZkNymTicketHandlerDebug,
};
use crate::config::persistence::{
    AuthenticatorPaths, GatewayTasksPaths, IpPacketRouterPaths, KeysPaths, NetworkRequesterPaths,
    NymNodePaths, ReplayProtectionPaths, ServiceProvidersPaths, WireguardPaths,
    DEFAULT_PRIMARY_X25519_SPHINX_KEY_FILENAME, DEFAULT_SECONDARY_X25519_SPHINX_KEY_FILENAME,
};
use crate::config::service_providers::{
    IpPacketRouter, IpPacketRouterDebug, NetworkRequester, NetworkRequesterDebug,
};
use crate::config::{
    gateway_tasks, service_providers, Config, GatewayTasksConfig, Host, Http, Mixnet, MixnetDebug,
    NodeModes, ReplayProtection, ReplayProtectionDebug, ServiceProvidersConfig, Verloc,
    VerlocDebug, Wireguard, DEFAULT_HTTP_PORT,
};
use crate::error::{KeyIOFailure, NymNodeError};
use crate::node::helpers::{get_current_rotation_id, load_key, store_key};
use crate::node::key_rotation::key::SphinxPrivateKey;
use celes::Country;
use clap::ValueEnum;
use nym_bin_common::logging::LoggingSettings;
use nym_client_core_config_types::DebugConfig as ClientDebugConfig;
use nym_config::defaults::DEFAULT_VERLOC_LISTENING_PORT;
use nym_config::helpers::{in6addr_any_init, inaddr_any};
use nym_config::{
    defaults::TICKETBOOK_VALIDITY_DAYS,
    read_config_from_toml_file,
    serde_helpers::{de_maybe_port, de_maybe_stringified},
};
use nym_crypto::asymmetric::x25519;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, instrument};
use url::Url;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardPathsV9 {
    pub private_diffie_hellman_key_file: PathBuf,
    pub public_diffie_hellman_key_file: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardV9 {
    /// Specifies whether the wireguard service is enabled on this node.
    pub enabled: bool,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `[::]:51822`
    pub bind_address: SocketAddr,

    /// Private IPv4 address of the wireguard gateway.
    /// default: `10.1.0.1`
    pub private_ipv4: Ipv4Addr,

    /// Private IPv6 address of the wireguard gateway.
    /// default: `fc01::1`
    pub private_ipv6: Ipv6Addr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv4.
    /// The maximum value for IPv4 is 32
    pub private_network_prefix_v4: u8,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv6.
    /// The maximum value for IPv6 is 128
    pub private_network_prefix_v6: u8,

    /// Paths for wireguard keys, client registries, etc.
    pub storage_paths: WireguardPathsV9,
}

// a temporary solution until all "types" are run at the same time
#[derive(Debug, Default, Serialize, Deserialize, ValueEnum, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum NodeModeV9 {
    #[default]
    #[clap(alias = "mix")]
    Mixnode,

    #[clap(alias = "entry", alias = "gateway")]
    EntryGateway,

    // to not break existing behaviour, this means exit capabilities AND entry capabilities
    #[clap(alias = "exit")]
    ExitGateway,

    // will start only SP needed for exit capabilities WITHOUT entry routing
    ExitProvidersOnly,
}

impl From<NodeModeV9> for NodeModes {
    fn from(config: NodeModeV9) -> Self {
        match config {
            NodeModeV9::Mixnode => *NodeModes::default().with_mixnode(),
            NodeModeV9::EntryGateway => *NodeModes::default().with_entry(),
            // in old version exit implied entry
            NodeModeV9::ExitGateway => *NodeModes::default().with_entry().with_exit(),
            NodeModeV9::ExitProvidersOnly => *NodeModes::default().with_exit(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct NodeModesV9 {
    /// Specifies whether this node can operate in a mixnode mode.
    pub mixnode: bool,

    /// Specifies whether this node can operate in an entry mode.
    pub entry: bool,

    /// Specifies whether this node can operate in an exit mode.
    pub exit: bool,
    // TODO: would it make sense to also put WG here for completion?
}

// TODO: this is very much a WIP. we need proper ssl certificate support here
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct HostV9 {
    /// Ip address(es) of this host, such as 1.1.1.1 that external clients will use for connections.
    /// If no values are provided, when this node gets included in the network,
    /// its ip addresses will be populated by whatever value is resolved by associated nym-api.
    pub public_ips: Vec<IpAddr>,

    /// Optional hostname of this node, for example nymtech.net.
    // TODO: this is temporary. to be replaced by pulling the data directly from the certs.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub hostname: Option<String>,

    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub location: Option<Country>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct MixnetDebugV9 {
    /// Specifies the duration of time this node is willing to delay a forward packet for.
    #[serde(with = "humantime_serde")]
    pub maximum_forward_packet_delay: Duration,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    #[serde(with = "humantime_serde")]
    pub initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    pub maximum_connection_buffer_size: usize,

    /// Specifies whether this node should **NOT** use noise protocol in the connections (currently not implemented)
    pub unsafe_disable_noise: bool,
}

impl MixnetDebugV9 {
    // given that genuine clients are using mean delay of 50ms,
    // the probability of them delaying for over 10s is 10^-87
    // which for all intents and purposes will never happen
    pub(crate) const DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY: Duration = Duration::from_secs(10);
    pub(crate) const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration =
        Duration::from_millis(10_000);
    pub(crate) const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration =
        Duration::from_millis(300_000);
    pub(crate) const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
    pub(crate) const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;
}

impl Default for MixnetDebugV9 {
    fn default() -> Self {
        MixnetDebugV9 {
            maximum_forward_packet_delay: Self::DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY,
            packet_forwarding_initial_backoff: Self::DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: Self::DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: Self::DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: Self::DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            // to be changed by @SW once the implementation is there
            unsafe_disable_noise: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixnetV9 {
    /// Address this node will bind to for listening for mixnet packets
    /// default: `[::]:1789`
    pub bind_address: SocketAddr,

    /// If applicable, custom port announced in the self-described API that other clients and nodes
    /// will use.
    /// Useful when the node is behind a proxy.
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_port: Option<u16>,

    /// Addresses to nym APIs from which the node gets the view of the network.
    pub nym_api_urls: Vec<Url>,

    /// Addresses to nyxd which the node uses to interact with the nyx chain.
    pub nyxd_urls: Vec<Url>,

    /// Settings for controlling replay detection
    pub replay_protection: ReplayProtectionV9,

    #[serde(default)]
    pub debug: MixnetDebugV9,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct ReplayProtectionV9 {
    /// Paths for current bloomfilters
    pub storage_paths: ReplayProtectionPathsV9,

    #[serde(default)]
    pub debug: ReplayProtectionDebugV9,
}

impl ReplayProtectionV9 {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        ReplayProtectionV9 {
            storage_paths: ReplayProtectionPathsV9::new(data_dir),
            debug: Default::default(),
        }
    }
}

pub const DEFAULT_RD_BLOOMFILTER_SUBDIR: &str = "replay-detection";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayProtectionPathsV9 {
    /// Path to the directory storing currently used bloomfilter(s).
    pub current_bloomfilters_directory: PathBuf,
}

impl ReplayProtectionPathsV9 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        ReplayProtectionPathsV9 {
            current_bloomfilters_directory: data_dir.as_ref().join(DEFAULT_RD_BLOOMFILTER_SUBDIR),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ReplayProtectionDebugV9 {
    /// Specifies whether this node should **NOT** use replay protection
    pub unsafe_disabled: bool,

    /// How long the processing task is willing to skip mutex acquisition before it will block the thread
    /// until it actually obtains it
    pub maximum_replay_detection_deferral: Duration,

    /// How many packets the processing task is willing to queue before it will block the thread
    /// until it obtains the mutex
    pub maximum_replay_detection_pending_packets: usize,

    /// Probability of false positives, fraction between 0 and 1 or a number indicating 1-in-p
    pub false_positive_rate: f64,

    /// Defines initial expected number of packets this node will process a second,
    /// so that an initial bloomfilter could be established.
    /// As the node is running and BF are cleared, the value will be adjusted dynamically
    pub initial_expected_packets_per_second: usize,

    /// Defines minimum expected number of packets this node will process a second
    /// when used for calculating the BF size after reset.
    /// This is to avoid degenerate cases where node receives 0 packets (because say it's misconfigured)
    /// and it constructs an empty bloomfilter.
    pub bloomfilter_minimum_packets_per_second_size: usize,

    /// Specifies the amount the bloomfilter size is going to get multiplied by after each reset.
    /// It's performed in case the traffic rates increase before the next bloomfilter update.
    pub bloomfilter_size_multiplier: f64,

    // NOTE: this field is temporary until replay detection bloomfilter rotation is tied
    // to key rotation
    /// Specifies how often the bloomfilter is cleared
    #[serde(with = "humantime_serde")]
    pub bloomfilter_reset_rate: Duration,

    /// Specifies how often the bloomfilter is flushed to disk for recovery in case of a crash
    #[serde(with = "humantime_serde")]
    pub bloomfilter_disk_flushing_rate: Duration,
}

impl ReplayProtectionDebugV9 {
    pub const DEFAULT_MAXIMUM_REPLAY_DETECTION_DEFERRAL: Duration = Duration::from_millis(50);

    pub const DEFAULT_MAXIMUM_REPLAY_DETECTION_PENDING_PACKETS: usize = 100;

    // 12% (completely arbitrary)
    pub const DEFAULT_BLOOMFILTER_SIZE_MULTIPLIER: f64 = 1.12;

    // 10^-5
    pub const DEFAULT_REPLAY_DETECTION_FALSE_POSITIVE_RATE: f64 = 1e-5;

    // 25h (key rotation will be happening every 24h + 1h of overlap)
    pub const DEFAULT_REPLAY_DETECTION_BF_RESET_RATE: Duration = Duration::from_secs(25 * 60 * 60);

    // we must have some reasonable balance between losing values and trashing the disk.
    // since on average HDD it would take ~30s to save a 2GB bloomfilter
    pub const DEFAULT_BF_DISK_FLUSHING_RATE: Duration = Duration::from_secs(10 * 60);

    // this value will have to be adjusted in the future
    pub const DEFAULT_INITIAL_EXPECTED_PACKETS_PER_SECOND: usize = 2000;

    pub const DEFAULT_BLOOMFILTER_MINIMUM_PACKETS_PER_SECOND_SIZE: usize = 200;
}

impl Default for ReplayProtectionDebugV9 {
    fn default() -> Self {
        ReplayProtectionDebugV9 {
            unsafe_disabled: false,
            maximum_replay_detection_deferral: Self::DEFAULT_MAXIMUM_REPLAY_DETECTION_DEFERRAL,
            maximum_replay_detection_pending_packets:
                Self::DEFAULT_MAXIMUM_REPLAY_DETECTION_PENDING_PACKETS,
            false_positive_rate: Self::DEFAULT_REPLAY_DETECTION_FALSE_POSITIVE_RATE,
            initial_expected_packets_per_second: Self::DEFAULT_INITIAL_EXPECTED_PACKETS_PER_SECOND,
            bloomfilter_minimum_packets_per_second_size:
                Self::DEFAULT_BLOOMFILTER_MINIMUM_PACKETS_PER_SECOND_SIZE,
            bloomfilter_size_multiplier: Self::DEFAULT_BLOOMFILTER_SIZE_MULTIPLIER,
            bloomfilter_reset_rate: Self::DEFAULT_REPLAY_DETECTION_BF_RESET_RATE,
            bloomfilter_disk_flushing_rate: Self::DEFAULT_BF_DISK_FLUSHING_RATE,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeysPathsV9 {
    /// Path to file containing ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing x25519 sphinx private key.
    pub private_x25519_sphinx_key_file: PathBuf,

    /// Path to file containing x25519 sphinx public key.
    pub public_x25519_sphinx_key_file: PathBuf,

    /// Path to file containing x25519 noise private key.
    pub private_x25519_noise_key_file: PathBuf,

    /// Path to file containing x25519 noise public key.
    pub public_x25519_noise_key_file: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NymNodePathsV9 {
    pub keys: KeysPathsV9,

    /// Path to a file containing basic node description: human-readable name, website, details, etc.
    pub description: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct HttpV9 {
    /// Socket address this node will use for binding its http API.
    /// default: `[::]:8080`
    pub bind_address: SocketAddr,

    /// Path to assets directory of custom landing page of this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub landing_page_assets_path: Option<PathBuf>,

    /// An optional bearer token for accessing certain http endpoints.
    /// Currently only used for obtaining mixnode's stats.
    #[serde(default)]
    pub access_token: Option<String>,

    /// Specify whether basic system information should be exposed.
    /// default: true
    pub expose_system_info: bool,

    /// Specify whether basic system hardware information should be exposed.
    /// This option is superseded by `expose_system_info`
    /// default: true
    pub expose_system_hardware: bool,

    /// Specify whether detailed system crypto hardware information should be exposed.
    /// This option is superseded by `expose_system_hardware`
    /// default: true
    pub expose_crypto_hardware: bool,

    /// Specify the cache ttl of the node load.
    /// default: 30s
    #[serde(with = "humantime_serde")]
    pub node_load_cache_ttl: Duration,
}

impl HttpV9 {
    pub const DEFAULT_NODE_LOAD_CACHE_TTL: Duration = Duration::from_secs(30);
}

impl Default for HttpV9 {
    fn default() -> Self {
        HttpV9 {
            bind_address: SocketAddr::new(inaddr_any(), DEFAULT_HTTP_PORT),
            landing_page_assets_path: None,
            access_token: None,
            expose_system_info: true,
            expose_system_hardware: true,
            expose_crypto_hardware: true,
            node_load_cache_ttl: Self::DEFAULT_NODE_LOAD_CACHE_TTL,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixnodePathsV9 {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugV9 {
    /// Delay between each subsequent node statistics being logged to the console
    #[serde(with = "humantime_serde")]
    pub node_stats_logging_delay: Duration,

    /// Delay between each subsequent node statistics being updated
    #[serde(with = "humantime_serde")]
    pub node_stats_updating_delay: Duration,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerlocDebugV9 {
    /// Specifies number of echo packets sent to each node during a measurement run.
    pub packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the connection to get established.
    #[serde(with = "humantime_serde")]
    pub connection_timeout: Duration,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    #[serde(with = "humantime_serde")]
    pub packet_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    #[serde(with = "humantime_serde")]
    pub delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    pub tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    #[serde(with = "humantime_serde")]
    pub testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    #[serde(with = "humantime_serde")]
    pub retry_timeout: Duration,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerlocV9 {
    /// Socket address this node will use for binding its verloc API.
    /// default: `[::]:1790`
    pub bind_address: SocketAddr,

    /// If applicable, custom port announced in the self-described API that other clients and nodes
    /// will use.
    /// Useful when the node is behind a proxy.
    #[serde(deserialize_with = "de_maybe_port")]
    #[serde(default)]
    pub announce_port: Option<u16>,

    #[serde(default)]
    pub debug: VerlocDebugV9,
}

impl VerlocV9 {
    pub const DEFAULT_VERLOC_PORT: u16 = DEFAULT_VERLOC_LISTENING_PORT;
}

impl Default for VerlocV9 {
    fn default() -> Self {
        VerlocV9 {
            bind_address: SocketAddr::new(in6addr_any_init(), Self::DEFAULT_VERLOC_PORT),
            announce_port: None,
            debug: Default::default(),
        }
    }
}

impl VerlocDebugV9 {
    const DEFAULT_PACKETS_PER_NODE: usize = 100;
    const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
    const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
    const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
    const DEFAULT_BATCH_SIZE: usize = 50;
    const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
    const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);
}

impl Default for VerlocDebugV9 {
    fn default() -> Self {
        VerlocDebugV9 {
            packets_per_node: Self::DEFAULT_PACKETS_PER_NODE,
            connection_timeout: Self::DEFAULT_CONNECTION_TIMEOUT,
            packet_timeout: Self::DEFAULT_PACKET_TIMEOUT,
            delay_between_packets: Self::DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: Self::DEFAULT_BATCH_SIZE,
            testing_interval: Self::DEFAULT_TESTING_INTERVAL,
            retry_timeout: Self::DEFAULT_RETRY_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixnodeConfigV9 {
    pub storage_paths: MixnodePathsV9,

    pub verloc: VerlocV9,

    #[serde(default)]
    pub debug: DebugV9,
}

impl DebugV9 {
    const DEFAULT_NODE_STATS_LOGGING_DELAY: Duration = Duration::from_millis(60_000);
    const DEFAULT_NODE_STATS_UPDATING_DELAY: Duration = Duration::from_millis(30_000);
}

impl Default for DebugV9 {
    fn default() -> Self {
        DebugV9 {
            node_stats_logging_delay: Self::DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: Self::DEFAULT_NODE_STATS_UPDATING_DELAY,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayPathsV9 {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys and available client bandwidths.
    pub clients_storage: PathBuf,

    pub stats_storage: PathBuf,

    /// Path to file containing cosmos account mnemonic used for zk-nym redemption.
    pub cosmos_mnemonic: PathBuf,

    pub authenticator: AuthenticatorPathsV9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ZkNymTicketHandlerDebugV9 {
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

impl ZkNymTicketHandlerDebugV9 {
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

impl Default for ZkNymTicketHandlerDebugV9 {
    fn default() -> Self {
        ZkNymTicketHandlerDebugV9 {
            revocation_bandwidth_penalty: Self::DEFAULT_REVOCATION_BANDWIDTH_PENALTY,
            pending_poller: Self::DEFAULT_PENDING_POLLER,
            minimum_api_quorum: Self::DEFAULT_MINIMUM_API_QUORUM,
            minimum_redemption_tickets: Self::DEFAULT_MINIMUM_REDEMPTION_TICKETS,
            maximum_time_between_redemption: Self::default_maximum_time_between_redemption(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayConfigDebugV9 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
    pub zk_nym_tickets: ZkNymTicketHandlerDebugV9,
}

impl EntryGatewayConfigDebugV9 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for EntryGatewayConfigDebugV9 {
    fn default() -> Self {
        EntryGatewayConfigDebugV9 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            zk_nym_tickets: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayConfigV9 {
    pub storage_paths: EntryGatewayPathsV9,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet
    /// or if it also accepts non-paying clients
    pub enforce_zk_nyms: bool,

    /// Socket address this node will use for binding its client websocket API.
    /// default: `[::]:9000`
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
    pub debug: EntryGatewayConfigDebugV9,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkRequesterPathsV9 {
    /// Path to file containing network requester ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing network requester ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing network requester x25519 diffie hellman private key.
    pub private_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing network requester x25519 diffie hellman public key.
    pub public_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,

    /// Normally this is a path to the file containing information about gateways used by this client,
    /// i.e. details such as their public keys, owner addresses or the network information.
    /// but in this case it just has the basic information of "we're using custom gateway".
    /// Due to how clients are started up, this file has to exist.
    pub gateway_registrations: PathBuf,
    // it's possible we might have to add credential storage here for return tickets
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IpPacketRouterPathsV9 {
    /// Path to file containing ip packet router ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ip packet router ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ip packet router x25519 diffie hellman private key.
    pub private_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing ip packet router x25519 diffie hellman public key.
    pub public_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,

    /// Normally this is a path to the file containing information about gateways used by this client,
    /// i.e. details such as their public keys, owner addresses or the network information.
    /// but in this case it just has the basic information of "we're using custom gateway".
    /// Due to how clients are started up, this file has to exist.
    pub gateway_registrations: PathBuf,
    // it's possible we might have to add credential storage here for return tickets
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatorPathsV9 {
    /// Path to file containing authenticator ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing authenticator ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing authenticator x25519 diffie hellman private key.
    pub private_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing authenticator x25519 diffie hellman public key.
    pub public_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,

    /// Normally this is a path to the file containing information about gateways used by this client,
    /// i.e. details such as their public keys, owner addresses or the network information.
    /// but in this case it just has the basic information of "we're using custom gateway".
    /// Due to how clients are started up, this file has to exist.
    pub gateway_registrations: PathBuf,
    // it's possible we might have to add credential storage here for return tickets
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayPathsV9 {
    pub clients_storage: PathBuf,

    pub stats_storage: PathBuf,

    pub network_requester: NetworkRequesterPathsV9,

    pub ip_packet_router: IpPacketRouterPathsV9,

    pub authenticator: AuthenticatorPathsV9,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AuthenticatorV9 {
    #[serde(default)]
    pub debug: AuthenticatorDebugV9,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct AuthenticatorDebugV9 {
    /// Specifies whether authenticator service is enabled in this process.
    /// This is only here for debugging purposes as exit gateway should always run
    /// the authenticator.
    pub enabled: bool,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting client_debug.traffic.disable_main_poisson_packet_distribution = true
    /// (or is it (?))
    pub disable_poisson_rate: bool,

    /// Shared detailed client configuration options
    #[serde(flatten)]
    pub client_debug: ClientDebugConfig,
}

impl Default for AuthenticatorDebugV9 {
    fn default() -> Self {
        AuthenticatorDebugV9 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AuthenticatorV9 {
    fn default() -> Self {
        AuthenticatorV9 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct IpPacketRouterDebugV9 {
    /// Specifies whether ip packet routing service is enabled in this process.
    /// This is only here for debugging purposes as exit gateway should always run **both**
    /// network requester and an ip packet router.
    pub enabled: bool,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting client_debug.traffic.disable_main_poisson_packet_distribution = true
    /// (or is it (?))
    pub disable_poisson_rate: bool,

    /// Shared detailed client configuration options
    #[serde(flatten)]
    pub client_debug: ClientDebugConfig,
}

impl Default for IpPacketRouterDebugV9 {
    fn default() -> Self {
        IpPacketRouterDebugV9 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct IpPacketRouterV9 {
    #[serde(default)]
    pub debug: IpPacketRouterDebugV9,
}

#[allow(clippy::derivable_impls)]
impl Default for IpPacketRouterV9 {
    fn default() -> Self {
        IpPacketRouterV9 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequesterDebugV9 {
    /// Specifies whether network requester service is enabled in this process.
    /// This is only here for debugging purposes as exit gateway should always run **both**
    /// network requester and an ip packet router.
    pub enabled: bool,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting client_debug.traffic.disable_main_poisson_packet_distribution = true
    /// (or is it (?))
    pub disable_poisson_rate: bool,

    /// Shared detailed client configuration options
    #[serde(flatten)]
    pub client_debug: ClientDebugConfig,
}

impl Default for NetworkRequesterDebugV9 {
    fn default() -> Self {
        NetworkRequesterDebugV9 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequesterV9 {
    #[serde(default)]
    pub debug: NetworkRequesterDebugV9,
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequesterV9 {
    fn default() -> Self {
        NetworkRequesterV9 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayDebugV9 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
}

impl ExitGatewayDebugV9 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for ExitGatewayDebugV9 {
    fn default() -> Self {
        ExitGatewayDebugV9 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayConfigV9 {
    pub storage_paths: ExitGatewayPathsV9,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Url,

    pub network_requester: NetworkRequesterV9,

    pub ip_packet_router: IpPacketRouterV9,

    #[serde(default)]
    pub debug: ExitGatewayDebugV9,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksPathsV9 {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys, available client bandwidths and wireguard peers.
    pub clients_storage: PathBuf,

    /// Path to sqlite database containing all persistent stats data.
    pub stats_storage: PathBuf,

    /// Path to file containing cosmos account mnemonic used for zk-nym redemption.
    pub cosmos_mnemonic: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StaleMessageDebugV9 {
    /// Specifies how often the clean-up task should check for stale data.
    #[serde(with = "humantime_serde")]
    pub cleaner_run_interval: Duration,

    /// Specifies maximum age of stored messages before they are removed from the storage
    #[serde(with = "humantime_serde")]
    pub max_age: Duration,
}

impl StaleMessageDebugV9 {
    const DEFAULT_STALE_MESSAGES_CLEANER_RUN_INTERVAL: Duration = Duration::from_secs(60 * 60);
    const DEFAULT_STALE_MESSAGES_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
}

impl Default for StaleMessageDebugV9 {
    fn default() -> Self {
        StaleMessageDebugV9 {
            cleaner_run_interval: Self::DEFAULT_STALE_MESSAGES_CLEANER_RUN_INTERVAL,
            max_age: Self::DEFAULT_STALE_MESSAGES_MAX_AGE,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClientBandwidthDebugV9 {
    /// Defines maximum delay between client bandwidth information being flushed to the persistent storage.
    pub max_flushing_rate: Duration,

    /// Defines a maximum change in client bandwidth before it gets flushed to the persistent storage.
    pub max_delta_flushing_amount: i64,
}

impl ClientBandwidthDebugV9 {
    const DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE: Duration = Duration::from_millis(5);
    const DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT: i64 = 512 * 1024; // 512kB
}

impl Default for ClientBandwidthDebugV9 {
    fn default() -> Self {
        ClientBandwidthDebugV9 {
            max_flushing_rate: Self::DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE,
            max_delta_flushing_amount: Self::DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct GatewayTasksConfigDebugV9 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,

    /// The maximum number of client connections the gateway will keep open at once.
    pub maximum_open_connections: usize,

    /// Specifies the minimum performance of mixnodes in the network that are to be used in internal topologies
    /// of the services providers
    pub minimum_mix_performance: u8,

    /// Defines the timestamp skew of a signed authentication request before it's deemed too excessive to process.
    #[serde(alias = "maximum_auth_request_age")]
    pub max_request_timestamp_skew: Duration,

    pub stale_messages: StaleMessageDebugV9,

    pub client_bandwidth: ClientBandwidthDebugV9,

    pub zk_nym_tickets: ZkNymTicketHandlerDebugV9,
}

impl GatewayTasksConfigDebugV9 {
    pub const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
    pub const DEFAULT_MINIMUM_MIX_PERFORMANCE: u8 = 50;
    pub const DEFAULT_MAXIMUM_AUTH_REQUEST_TIMESTAMP_SKEW: Duration = Duration::from_secs(120);
    pub const DEFAULT_MAXIMUM_OPEN_CONNECTIONS: usize = 8192;
}

impl Default for GatewayTasksConfigDebugV9 {
    fn default() -> Self {
        GatewayTasksConfigDebugV9 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            maximum_open_connections: Self::DEFAULT_MAXIMUM_OPEN_CONNECTIONS,
            max_request_timestamp_skew: Self::DEFAULT_MAXIMUM_AUTH_REQUEST_TIMESTAMP_SKEW,
            minimum_mix_performance: Self::DEFAULT_MINIMUM_MIX_PERFORMANCE,
            stale_messages: Default::default(),
            client_bandwidth: Default::default(),
            zk_nym_tickets: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksConfigV9 {
    pub storage_paths: GatewayTasksPathsV9,

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
    pub debug: GatewayTasksConfigDebugV9,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvidersPathsV9 {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys, available client bandwidths and wireguard peers.
    pub clients_storage: PathBuf,

    /// Path to sqlite database containing all persistent stats data.
    pub stats_storage: PathBuf,

    pub network_requester: NetworkRequesterPathsV9,

    pub ip_packet_router: IpPacketRouterPathsV9,

    pub authenticator: AuthenticatorPathsV9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvidersConfigDebugV9 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
}

impl ServiceProvidersConfigDebugV9 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for ServiceProvidersConfigDebugV9 {
    fn default() -> Self {
        ServiceProvidersConfigDebugV9 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvidersConfigV9 {
    pub storage_paths: ServiceProvidersPathsV9,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Url,

    pub network_requester: NetworkRequesterV9,

    pub ip_packet_router: IpPacketRouterV9,

    pub authenticator: AuthenticatorV9,

    #[serde(default)]
    pub debug: ServiceProvidersConfigDebugV9,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsConfigV9 {
    #[serde(default)]
    pub debug: MetricsDebugV9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsDebugV9 {
    /// Specify whether running statistics of this node should be logged to the console.
    pub log_stats_to_console: bool,

    /// Specify the rate of which the metrics aggregator should call the `on_update` methods of all its registered handlers.
    #[serde(with = "humantime_serde")]
    pub aggregator_update_rate: Duration,

    /// Specify the target rate of clearing old stale mixnet metrics.
    #[serde(with = "humantime_serde")]
    pub stale_mixnet_metrics_cleaner_rate: Duration,

    /// Specify the target rate of updating global prometheus counters.
    #[serde(with = "humantime_serde")]
    pub global_prometheus_counters_update_rate: Duration,

    /// Specify the target rate of updating egress packets pending delivery counter.
    #[serde(with = "humantime_serde")]
    pub pending_egress_packets_update_rate: Duration,

    /// Specify the rate of updating clients sessions
    #[serde(with = "humantime_serde")]
    pub clients_sessions_update_rate: Duration,

    /// If console logging is enabled, specify the interval at which that happens
    #[serde(with = "humantime_serde")]
    pub console_logging_update_interval: Duration,

    /// Specify the update rate of running stats for the legacy `/metrics/mixing` endpoint
    #[serde(with = "humantime_serde")]
    pub legacy_mixing_metrics_update_rate: Duration,
}

impl MetricsDebugV9 {
    const DEFAULT_CONSOLE_LOGGING_INTERVAL: Duration = Duration::from_millis(60_000);
    const DEFAULT_LEGACY_MIXING_UPDATE_RATE: Duration = Duration::from_millis(30_000);
    const DEFAULT_AGGREGATOR_UPDATE_RATE: Duration = Duration::from_secs(5);
    const DEFAULT_STALE_MIXNET_METRICS_UPDATE_RATE: Duration = Duration::from_secs(3600);
    const DEFAULT_CLIENT_SESSIONS_UPDATE_RATE: Duration = Duration::from_secs(3600);
    const GLOBAL_PROMETHEUS_COUNTERS_UPDATE_INTERVAL: Duration = Duration::from_secs(30);
    const DEFAULT_PENDING_EGRESS_PACKETS_UPDATE_RATE: Duration = Duration::from_secs(30);
}

impl Default for MetricsDebugV9 {
    fn default() -> Self {
        MetricsDebugV9 {
            log_stats_to_console: true,
            console_logging_update_interval: Self::DEFAULT_CONSOLE_LOGGING_INTERVAL,
            legacy_mixing_metrics_update_rate: Self::DEFAULT_LEGACY_MIXING_UPDATE_RATE,
            aggregator_update_rate: Self::DEFAULT_AGGREGATOR_UPDATE_RATE,
            stale_mixnet_metrics_cleaner_rate: Self::DEFAULT_STALE_MIXNET_METRICS_UPDATE_RATE,
            global_prometheus_counters_update_rate:
                Self::GLOBAL_PROMETHEUS_COUNTERS_UPDATE_INTERVAL,
            pending_egress_packets_update_rate: Self::DEFAULT_PENDING_EGRESS_PACKETS_UPDATE_RATE,
            clients_sessions_update_rate: Self::DEFAULT_CLIENT_SESSIONS_UPDATE_RATE,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettingsV9 {
    // well, we need to implement something here at some point...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV9 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModesV9,

    pub host: HostV9,

    pub mixnet: MixnetV9,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePathsV9,

    #[serde(default)]
    pub http: HttpV9,

    #[serde(default)]
    pub verloc: VerlocV9,

    pub wireguard: WireguardV9,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfigV9,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfigV9,

    #[serde(default)]
    pub metrics: MetricsConfigV9,

    #[serde(default)]
    pub logging: LoggingSettingsV9,

    #[serde(default)]
    pub debug: DebugV9,
}

impl ConfigV9 {
    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: ConfigV9 =
            read_config_from_toml_file(path).map_err(|source| NymNodeError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }
}

async fn upgrade_sphinx_key(old_cfg: &ConfigV9) -> Result<(PathBuf, PathBuf), NymNodeError> {
    // we mark the current sphinx key as the primary and attach the current rotation id
    let rotation_id =
        get_current_rotation_id(&old_cfg.mixnet.nym_api_urls, &old_cfg.mixnet.nyxd_urls).await?;

    let current_sphinx_key_path = &old_cfg.storage_paths.keys.private_x25519_sphinx_key_file;
    let current_pubkey_path = &old_cfg.storage_paths.keys.public_x25519_sphinx_key_file;

    let current_sphinx_key: x25519::PrivateKey =
        load_key(current_sphinx_key_path, "sphinx private key")?;

    let keys_dir = current_sphinx_key_path
        .parent()
        .ok_or(NymNodeError::DataDirDerivationFailure)?;

    let primary_key_path = keys_dir.join(DEFAULT_PRIMARY_X25519_SPHINX_KEY_FILENAME);
    let secondary_key_path = keys_dir.join(DEFAULT_SECONDARY_X25519_SPHINX_KEY_FILENAME);

    let primary_key = SphinxPrivateKey::import(current_sphinx_key, rotation_id);
    store_key(&primary_key, &primary_key_path, "sphinx private key")?;

    // no point in keeping the old sphinx files
    fs::remove_file(current_sphinx_key_path).map_err(|err| KeyIOFailure::KeyRemovalFailure {
        key: "sphinx private key".to_string(),
        path: current_sphinx_key_path.clone(),
        err,
    })?;
    fs::remove_file(current_pubkey_path).map_err(|err| KeyIOFailure::KeyRemovalFailure {
        key: "sphinx public key".to_string(),
        path: current_pubkey_path.clone(),
        err,
    })?;

    Ok((primary_key_path, secondary_key_path))
}

#[instrument(skip_all)]
pub async fn try_upgrade_config_v9<P: AsRef<Path>>(
    path: P,
    prev_config: Option<ConfigV9>,
) -> Result<Config, NymNodeError> {
    debug!("attempting to load v9 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV9::read_from_path(&path)?
    };

    let (primary_x25519_sphinx_key_file, secondary_x25519_sphinx_key_file) =
        upgrade_sphinx_key(&old_cfg).await?;

    let cfg = Config {
        save_path: old_cfg.save_path,
        id: old_cfg.id,
        modes: NodeModes {
            mixnode: old_cfg.modes.mixnode,
            entry: old_cfg.modes.entry,
            exit: old_cfg.modes.exit,
        },
        host: Host {
            public_ips: old_cfg.host.public_ips,
            hostname: old_cfg.host.hostname,
            location: old_cfg.host.location,
        },
        mixnet: Mixnet {
            bind_address: old_cfg.mixnet.bind_address,
            announce_port: old_cfg.mixnet.announce_port,
            nym_api_urls: old_cfg.mixnet.nym_api_urls,
            nyxd_urls: old_cfg.mixnet.nyxd_urls,
            replay_protection: ReplayProtection {
                storage_paths: ReplayProtectionPaths {
                    current_bloomfilters_directory: old_cfg
                        .mixnet
                        .replay_protection
                        .storage_paths
                        .current_bloomfilters_directory,
                },
                debug: ReplayProtectionDebug {
                    unsafe_disabled: old_cfg.mixnet.replay_protection.debug.unsafe_disabled,
                    maximum_replay_detection_deferral: old_cfg
                        .mixnet
                        .replay_protection
                        .debug
                        .maximum_replay_detection_deferral,
                    maximum_replay_detection_pending_packets: old_cfg
                        .mixnet
                        .replay_protection
                        .debug
                        .maximum_replay_detection_pending_packets,
                    false_positive_rate: old_cfg.mixnet.replay_protection.debug.false_positive_rate,
                    initial_expected_packets_per_second: old_cfg
                        .mixnet
                        .replay_protection
                        .debug
                        .initial_expected_packets_per_second,
                    bloomfilter_minimum_packets_per_second_size: old_cfg
                        .mixnet
                        .replay_protection
                        .debug
                        .bloomfilter_minimum_packets_per_second_size,
                    bloomfilter_size_multiplier: old_cfg
                        .mixnet
                        .replay_protection
                        .debug
                        .bloomfilter_size_multiplier,
                    bloomfilter_disk_flushing_rate: old_cfg
                        .mixnet
                        .replay_protection
                        .debug
                        .bloomfilter_disk_flushing_rate,
                },
            },
            key_rotation: Default::default(),
            debug: MixnetDebug {
                maximum_forward_packet_delay: old_cfg.mixnet.debug.maximum_forward_packet_delay,
                packet_forwarding_initial_backoff: old_cfg
                    .mixnet
                    .debug
                    .packet_forwarding_initial_backoff,
                packet_forwarding_maximum_backoff: old_cfg
                    .mixnet
                    .debug
                    .packet_forwarding_maximum_backoff,
                initial_connection_timeout: old_cfg.mixnet.debug.initial_connection_timeout,
                maximum_connection_buffer_size: old_cfg.mixnet.debug.maximum_connection_buffer_size,
                unsafe_disable_noise: old_cfg.mixnet.debug.unsafe_disable_noise,
            },
        },
        storage_paths: NymNodePaths {
            keys: KeysPaths {
                private_ed25519_identity_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_ed25519_identity_key_file,
                public_ed25519_identity_key_file: old_cfg
                    .storage_paths
                    .keys
                    .public_ed25519_identity_key_file,
                primary_x25519_sphinx_key_file,
                private_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_x25519_noise_key_file,
                public_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .public_x25519_noise_key_file,
                secondary_x25519_sphinx_key_file,
            },
            description: old_cfg.storage_paths.description,
        },
        http: Http {
            bind_address: old_cfg.http.bind_address,
            landing_page_assets_path: old_cfg.http.landing_page_assets_path,
            access_token: old_cfg.http.access_token,
            expose_system_info: old_cfg.http.expose_system_info,
            expose_system_hardware: old_cfg.http.expose_system_hardware,
            expose_crypto_hardware: old_cfg.http.expose_crypto_hardware,
            node_load_cache_ttl: old_cfg.http.node_load_cache_ttl,
        },
        verloc: Verloc {
            bind_address: old_cfg.verloc.bind_address,
            announce_port: old_cfg.verloc.announce_port,
            debug: VerlocDebug {
                packets_per_node: old_cfg.verloc.debug.packets_per_node,
                connection_timeout: old_cfg.verloc.debug.connection_timeout,
                packet_timeout: old_cfg.verloc.debug.packet_timeout,
                delay_between_packets: old_cfg.verloc.debug.delay_between_packets,
                tested_nodes_batch_size: old_cfg.verloc.debug.tested_nodes_batch_size,
                testing_interval: old_cfg.verloc.debug.testing_interval,
                retry_timeout: old_cfg.verloc.debug.retry_timeout,
            },
        },
        wireguard: Wireguard {
            enabled: old_cfg.wireguard.enabled,
            bind_address: old_cfg.wireguard.bind_address,
            private_ipv4: old_cfg.wireguard.private_ipv4,
            private_ipv6: old_cfg.wireguard.private_ipv6,
            announced_port: old_cfg.wireguard.announced_port,
            private_network_prefix_v4: old_cfg.wireguard.private_network_prefix_v4,
            private_network_prefix_v6: old_cfg.wireguard.private_network_prefix_v6,
            storage_paths: WireguardPaths {
                private_diffie_hellman_key_file: old_cfg
                    .wireguard
                    .storage_paths
                    .private_diffie_hellman_key_file,
                public_diffie_hellman_key_file: old_cfg
                    .wireguard
                    .storage_paths
                    .public_diffie_hellman_key_file,
            },
        },
        gateway_tasks: GatewayTasksConfig {
            storage_paths: GatewayTasksPaths {
                clients_storage: old_cfg.gateway_tasks.storage_paths.clients_storage,
                stats_storage: old_cfg.gateway_tasks.storage_paths.stats_storage,
                cosmos_mnemonic: old_cfg.gateway_tasks.storage_paths.cosmos_mnemonic,
            },
            enforce_zk_nyms: old_cfg.gateway_tasks.enforce_zk_nyms,
            ws_bind_address: old_cfg.gateway_tasks.ws_bind_address,
            announce_ws_port: old_cfg.gateway_tasks.announce_ws_port,
            announce_wss_port: old_cfg.gateway_tasks.announce_wss_port,
            debug: gateway_tasks::Debug {
                message_retrieval_limit: old_cfg.gateway_tasks.debug.message_retrieval_limit,
                maximum_open_connections: old_cfg.gateway_tasks.debug.maximum_open_connections,
                minimum_mix_performance: old_cfg.gateway_tasks.debug.minimum_mix_performance,
                max_request_timestamp_skew: old_cfg.gateway_tasks.debug.max_request_timestamp_skew,
                stale_messages: StaleMessageDebug {
                    cleaner_run_interval: old_cfg
                        .gateway_tasks
                        .debug
                        .stale_messages
                        .cleaner_run_interval,
                    max_age: old_cfg.gateway_tasks.debug.stale_messages.max_age,
                },
                client_bandwidth: ClientBandwidthDebug {
                    max_flushing_rate: old_cfg
                        .gateway_tasks
                        .debug
                        .client_bandwidth
                        .max_flushing_rate,
                    max_delta_flushing_amount: old_cfg
                        .gateway_tasks
                        .debug
                        .client_bandwidth
                        .max_delta_flushing_amount,
                },
                zk_nym_tickets: ZkNymTicketHandlerDebug {
                    revocation_bandwidth_penalty: old_cfg
                        .gateway_tasks
                        .debug
                        .zk_nym_tickets
                        .revocation_bandwidth_penalty,
                    pending_poller: old_cfg.gateway_tasks.debug.zk_nym_tickets.pending_poller,
                    minimum_api_quorum: old_cfg
                        .gateway_tasks
                        .debug
                        .zk_nym_tickets
                        .minimum_api_quorum,
                    minimum_redemption_tickets: old_cfg
                        .gateway_tasks
                        .debug
                        .zk_nym_tickets
                        .minimum_redemption_tickets,
                    maximum_time_between_redemption: old_cfg
                        .gateway_tasks
                        .debug
                        .zk_nym_tickets
                        .maximum_time_between_redemption,
                },
            },
        },
        service_providers: ServiceProvidersConfig {
            storage_paths: ServiceProvidersPaths {
                clients_storage: old_cfg.service_providers.storage_paths.clients_storage,
                stats_storage: old_cfg.service_providers.storage_paths.stats_storage,
                network_requester: NetworkRequesterPaths {
                    private_ed25519_identity_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .network_requester
                        .private_ed25519_identity_key_file,
                    public_ed25519_identity_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .network_requester
                        .public_ed25519_identity_key_file,
                    private_x25519_diffie_hellman_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .network_requester
                        .private_x25519_diffie_hellman_key_file,
                    public_x25519_diffie_hellman_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .network_requester
                        .public_x25519_diffie_hellman_key_file,
                    ack_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .network_requester
                        .ack_key_file,
                    reply_surb_database: old_cfg
                        .service_providers
                        .storage_paths
                        .network_requester
                        .reply_surb_database,
                    gateway_registrations: old_cfg
                        .service_providers
                        .storage_paths
                        .network_requester
                        .gateway_registrations,
                },
                ip_packet_router: IpPacketRouterPaths {
                    private_ed25519_identity_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .ip_packet_router
                        .private_ed25519_identity_key_file,
                    public_ed25519_identity_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .ip_packet_router
                        .public_ed25519_identity_key_file,
                    private_x25519_diffie_hellman_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .ip_packet_router
                        .private_x25519_diffie_hellman_key_file,
                    public_x25519_diffie_hellman_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .ip_packet_router
                        .public_x25519_diffie_hellman_key_file,
                    ack_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .ip_packet_router
                        .ack_key_file,
                    reply_surb_database: old_cfg
                        .service_providers
                        .storage_paths
                        .ip_packet_router
                        .reply_surb_database,
                    gateway_registrations: old_cfg
                        .service_providers
                        .storage_paths
                        .ip_packet_router
                        .gateway_registrations,
                },
                authenticator: AuthenticatorPaths {
                    private_ed25519_identity_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .authenticator
                        .private_ed25519_identity_key_file,
                    public_ed25519_identity_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .authenticator
                        .public_ed25519_identity_key_file,
                    private_x25519_diffie_hellman_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .authenticator
                        .private_x25519_diffie_hellman_key_file,
                    public_x25519_diffie_hellman_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .authenticator
                        .public_x25519_diffie_hellman_key_file,
                    ack_key_file: old_cfg
                        .service_providers
                        .storage_paths
                        .authenticator
                        .ack_key_file,
                    reply_surb_database: old_cfg
                        .service_providers
                        .storage_paths
                        .authenticator
                        .reply_surb_database,
                    gateway_registrations: old_cfg
                        .service_providers
                        .storage_paths
                        .authenticator
                        .gateway_registrations,
                },
            },
            open_proxy: old_cfg.service_providers.open_proxy,
            upstream_exit_policy_url: old_cfg.service_providers.upstream_exit_policy_url,
            network_requester: NetworkRequester {
                debug: NetworkRequesterDebug {
                    enabled: old_cfg.service_providers.network_requester.debug.enabled,
                    disable_poisson_rate: old_cfg
                        .service_providers
                        .network_requester
                        .debug
                        .disable_poisson_rate,
                    client_debug: old_cfg
                        .service_providers
                        .network_requester
                        .debug
                        .client_debug,
                },
            },
            ip_packet_router: IpPacketRouter {
                debug: IpPacketRouterDebug {
                    enabled: old_cfg.service_providers.ip_packet_router.debug.enabled,
                    disable_poisson_rate: old_cfg
                        .service_providers
                        .ip_packet_router
                        .debug
                        .disable_poisson_rate,
                    client_debug: old_cfg
                        .service_providers
                        .ip_packet_router
                        .debug
                        .client_debug,
                },
            },
            authenticator: Authenticator {
                debug: AuthenticatorDebug {
                    enabled: old_cfg.service_providers.authenticator.debug.enabled,
                    disable_poisson_rate: old_cfg
                        .service_providers
                        .authenticator
                        .debug
                        .disable_poisson_rate,
                    client_debug: old_cfg.service_providers.authenticator.debug.client_debug,
                },
            },
            debug: service_providers::Debug {
                message_retrieval_limit: old_cfg.service_providers.debug.message_retrieval_limit,
            },
        },
        metrics: Default::default(),
        logging: LoggingSettings {},
        debug: Default::default(),
    };
    Ok(cfg)
}
