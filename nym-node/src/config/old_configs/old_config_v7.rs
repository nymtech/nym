// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![allow(dead_code)]

use crate::config::authenticator::{Authenticator, AuthenticatorDebug};
use crate::config::gateway_tasks::ZkNymTicketHandlerDebug;
use crate::config::service_providers::{
    IpPacketRouter, IpPacketRouterDebug, NetworkRequester, NetworkRequesterDebug,
};
use crate::config::*;
use crate::error::{EntryGatewayError, NymNodeError};
use celes::Country;
use clap::ValueEnum;
use gateway_tasks::DEFAULT_WS_PORT;
use nym_client_core_config_types::{
    disk_persistence::{ClientKeysPaths, CommonClientPaths},
    DebugConfig as ClientDebugConfig,
};
use nym_config::defaults::{mainnet, var_names};
use nym_config::helpers::inaddr_any;
use nym_config::{
    defaults::TICKETBOOK_VALIDITY_DAYS,
    serde_helpers::{de_maybe_port, de_maybe_stringified},
};
use nym_config::{parse_urls, read_config_from_toml_file};
use persistence::*;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs, io};
use tracing::info;
use tracing::{debug, instrument};
use url::Url;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardPathsV7 {
    pub private_diffie_hellman_key_file: PathBuf,
    pub public_diffie_hellman_key_file: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardV7 {
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
    pub storage_paths: WireguardPathsV7,
}

// a temporary solution until all "types" are run at the same time
#[derive(Debug, Default, Serialize, Deserialize, ValueEnum, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum NodeModeV7 {
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

impl From<NodeModeV7> for NodeModes {
    fn from(config: NodeModeV7) -> Self {
        match config {
            NodeModeV7::Mixnode => *NodeModes::default().with_mixnode(),
            NodeModeV7::EntryGateway => *NodeModes::default().with_entry(),
            // in old version exit implied entry
            NodeModeV7::ExitGateway => *NodeModes::default().with_entry().with_exit(),
            NodeModeV7::ExitProvidersOnly => *NodeModes::default().with_exit(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct NodeModesV7 {
    /// Specifies whether this node can operate in a mixnode mode.
    pub mixnode: bool,

    /// Specifies whether this node can operate in an entry mode.
    pub entry: bool,

    /// Specifies whether this node can operate in an exit mode.
    pub exit: bool,
    // TODO: would it make sense to also put WG here for completion?
}

impl From<&[NodeModeV7]> for NodeModesV7 {
    fn from(modes: &[NodeModeV7]) -> Self {
        let mut out = NodeModesV7::default();
        for &mode in modes {
            out.with_mode(mode);
        }
        out
    }
}

impl NodeModesV7 {
    pub fn any_enabled(&self) -> bool {
        self.mixnode || self.entry || self.exit
    }

    pub fn standalone_exit(&self) -> bool {
        !self.mixnode && !self.entry && self.exit
    }

    pub fn with_mode(&mut self, mode: NodeModeV7) -> &mut Self {
        match mode {
            NodeModeV7::Mixnode => self.with_mixnode(),
            NodeModeV7::EntryGateway => self.with_entry(),
            NodeModeV7::ExitGateway => self.with_entry().with_exit(),
            NodeModeV7::ExitProvidersOnly => self.with_exit(),
        }
    }

    pub fn expects_final_hop_traffic(&self) -> bool {
        self.entry || self.exit
    }

    pub fn with_mixnode(&mut self) -> &mut Self {
        self.mixnode = true;
        self
    }

    pub fn with_entry(&mut self) -> &mut Self {
        self.entry = true;
        self
    }

    pub fn with_exit(&mut self) -> &mut Self {
        self.exit = true;
        self
    }
}

// TODO: this is very much a WIP. we need proper ssl certificate support here
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct HostV7 {
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
pub struct MixnetDebugV7 {
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

impl MixnetDebugV7 {
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

impl Default for MixnetDebugV7 {
    fn default() -> Self {
        MixnetDebugV7 {
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

impl Default for MixnetV7 {
    fn default() -> Self {
        // SAFETY:
        // our hardcoded values should always be valid
        #[allow(clippy::expect_used)]
        // is if there's anything set in the environment, otherwise fallback to mainnet
        let nym_api_urls = if let Ok(env_value) = env::var(var_names::NYM_API) {
            parse_urls(&env_value)
        } else {
            vec![mainnet::NYM_API.parse().expect("Invalid default API URL")]
        };

        #[allow(clippy::expect_used)]
        let nyxd_urls = if let Ok(env_value) = env::var(var_names::NYXD) {
            parse_urls(&env_value)
        } else {
            vec![mainnet::NYXD_URL.parse().expect("Invalid default nyxd URL")]
        };

        MixnetV7 {
            bind_address: SocketAddr::new(inaddr_any(), DEFAULT_MIXNET_PORT),
            announce_port: None,
            nym_api_urls,
            nyxd_urls,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct MixnetV7 {
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

    #[serde(default)]
    pub debug: MixnetDebugV7,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeysPathsV7 {
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

impl KeysPathsV7 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();

        KeysPathsV7 {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_sphinx_key_file: data_dir
                .join(DEFAULT_X25519_PRIVATE_SPHINX_KEY_FILENAME),
            public_x25519_sphinx_key_file: data_dir.join(DEFAULT_X25519_PUBLIC_SPHINX_KEY_FILENAME),
            private_x25519_noise_key_file: data_dir.join(DEFAULT_X25519_PRIVATE_NOISE_KEY_FILENAME),
            public_x25519_noise_key_file: data_dir.join(DEFAULT_X25519_PUBLIC_NOISE_KEY_FILENAME),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        )
    }

    pub fn x25519_sphinx_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_sphinx_key_file,
            &self.public_x25519_sphinx_key_file,
        )
    }

    pub fn x25519_noise_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_noise_key_file,
            &self.public_x25519_noise_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NymNodePathsV7 {
    pub keys: KeysPathsV7,

    /// Path to a file containing basic node description: human-readable name, website, details, etc.
    pub description: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct HttpV7 {
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
}

impl Default for HttpV7 {
    fn default() -> Self {
        HttpV7 {
            bind_address: SocketAddr::new(inaddr_any(), DEFAULT_HTTP_PORT),
            landing_page_assets_path: None,
            access_token: None,
            expose_system_info: true,
            expose_system_hardware: true,
            expose_crypto_hardware: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixnodePathsV7 {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugV7 {
    /// Delay between each subsequent node statistics being logged to the console
    #[serde(with = "humantime_serde")]
    pub node_stats_logging_delay: Duration,

    /// Delay between each subsequent node statistics being updated
    #[serde(with = "humantime_serde")]
    pub node_stats_updating_delay: Duration,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerlocDebugV7 {
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
pub struct VerlocV7 {
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
    pub debug: VerlocDebugV7,
}

impl VerlocV7 {
    pub const DEFAULT_VERLOC_PORT: u16 = DEFAULT_VERLOC_LISTENING_PORT;
}

impl Default for VerlocV7 {
    fn default() -> Self {
        VerlocV7 {
            bind_address: SocketAddr::new(in6addr_any_init(), Self::DEFAULT_VERLOC_PORT),
            announce_port: None,
            debug: Default::default(),
        }
    }
}

impl VerlocDebugV7 {
    const DEFAULT_PACKETS_PER_NODE: usize = 100;
    const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
    const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
    const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
    const DEFAULT_BATCH_SIZE: usize = 50;
    const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
    const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);
}

impl Default for VerlocDebugV7 {
    fn default() -> Self {
        VerlocDebugV7 {
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
pub struct MixnodeConfigV7 {
    pub storage_paths: MixnodePathsV7,

    pub verloc: VerlocV7,

    #[serde(default)]
    pub debug: DebugV7,
}

impl DebugV7 {
    const DEFAULT_NODE_STATS_LOGGING_DELAY: Duration = Duration::from_millis(60_000);
    const DEFAULT_NODE_STATS_UPDATING_DELAY: Duration = Duration::from_millis(30_000);
}

impl Default for DebugV7 {
    fn default() -> Self {
        DebugV7 {
            node_stats_logging_delay: Self::DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: Self::DEFAULT_NODE_STATS_UPDATING_DELAY,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayPathsV7 {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys and available client bandwidths.
    pub clients_storage: PathBuf,

    pub stats_storage: PathBuf,

    /// Path to file containing cosmos account mnemonic used for zk-nym redemption.
    pub cosmos_mnemonic: PathBuf,

    pub authenticator: AuthenticatorPathsV7,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ZkNymTicketHandlerDebugV7 {
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

impl ZkNymTicketHandlerDebugV7 {
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

impl Default for ZkNymTicketHandlerDebugV7 {
    fn default() -> Self {
        ZkNymTicketHandlerDebugV7 {
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
pub struct EntryGatewayConfigDebugV7 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
    pub zk_nym_tickets: ZkNymTicketHandlerDebugV7,
}

impl EntryGatewayConfigDebugV7 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for EntryGatewayConfigDebugV7 {
    fn default() -> Self {
        EntryGatewayConfigDebugV7 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            zk_nym_tickets: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayConfigV7 {
    pub storage_paths: EntryGatewayPathsV7,

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
    pub debug: EntryGatewayConfigDebugV7,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkRequesterPathsV7 {
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

impl NetworkRequesterPathsV7 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        NetworkRequesterPathsV7 {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_NR_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_NR_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_NR_PRIVATE_DH_KEY_FILENAME),
            public_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_NR_PUBLIC_DH_KEY_FILENAME),
            ack_key_file: data_dir.join(DEFAULT_NR_ACK_KEY_FILENAME),
            reply_surb_database: data_dir.join(DEFAULT_NR_REPLY_SURB_DB_FILENAME),
            gateway_registrations: data_dir.join(DEFAULT_NR_GATEWAYS_DB_FILENAME),
        }
    }

    pub fn to_common_client_paths(&self) -> CommonClientPaths {
        CommonClientPaths {
            keys: ClientKeysPaths {
                private_identity_key_file: self.private_ed25519_identity_key_file.clone(),
                public_identity_key_file: self.public_ed25519_identity_key_file.clone(),
                private_encryption_key_file: self.private_x25519_diffie_hellman_key_file.clone(),
                public_encryption_key_file: self.public_x25519_diffie_hellman_key_file.clone(),
                ack_key_file: self.ack_key_file.clone(),
            },
            gateway_registrations: self.gateway_registrations.clone(),

            // not needed for embedded providers
            credentials_database: Default::default(),
            reply_surb_database: self.reply_surb_database.clone(),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        )
    }

    pub fn x25519_diffie_hellman_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_diffie_hellman_key_file,
            &self.public_x25519_diffie_hellman_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IpPacketRouterPathsV7 {
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

impl IpPacketRouterPathsV7 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        IpPacketRouterPathsV7 {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_IPR_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_IPR_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_IPR_PRIVATE_DH_KEY_FILENAME),
            public_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_IPR_PUBLIC_DH_KEY_FILENAME),
            ack_key_file: data_dir.join(DEFAULT_IPR_ACK_KEY_FILENAME),
            reply_surb_database: data_dir.join(DEFAULT_IPR_REPLY_SURB_DB_FILENAME),
            gateway_registrations: data_dir.join(DEFAULT_IPR_GATEWAYS_DB_FILENAME),
        }
    }

    pub fn to_common_client_paths(&self) -> CommonClientPaths {
        CommonClientPaths {
            keys: ClientKeysPaths {
                private_identity_key_file: self.private_ed25519_identity_key_file.clone(),
                public_identity_key_file: self.public_ed25519_identity_key_file.clone(),
                private_encryption_key_file: self.private_x25519_diffie_hellman_key_file.clone(),
                public_encryption_key_file: self.public_x25519_diffie_hellman_key_file.clone(),
                ack_key_file: self.ack_key_file.clone(),
            },
            gateway_registrations: self.gateway_registrations.clone(),

            // not needed for embedded providers
            credentials_database: Default::default(),
            reply_surb_database: self.reply_surb_database.clone(),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        )
    }

    pub fn x25519_diffie_hellman_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_diffie_hellman_key_file,
            &self.public_x25519_diffie_hellman_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatorPathsV7 {
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

impl AuthenticatorPathsV7 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        AuthenticatorPathsV7 {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_AUTH_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_AUTH_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_AUTH_PRIVATE_DH_KEY_FILENAME),
            public_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_AUTH_PUBLIC_DH_KEY_FILENAME),
            ack_key_file: data_dir.join(DEFAULT_AUTH_ACK_KEY_FILENAME),
            reply_surb_database: data_dir.join(DEFAULT_AUTH_REPLY_SURB_DB_FILENAME),
            gateway_registrations: data_dir.join(DEFAULT_AUTH_GATEWAYS_DB_FILENAME),
        }
    }

    pub fn to_common_client_paths(&self) -> CommonClientPaths {
        CommonClientPaths {
            keys: ClientKeysPaths {
                private_identity_key_file: self.private_ed25519_identity_key_file.clone(),
                public_identity_key_file: self.public_ed25519_identity_key_file.clone(),
                private_encryption_key_file: self.private_x25519_diffie_hellman_key_file.clone(),
                public_encryption_key_file: self.public_x25519_diffie_hellman_key_file.clone(),
                ack_key_file: self.ack_key_file.clone(),
            },
            gateway_registrations: self.gateway_registrations.clone(),

            // not needed for embedded providers
            credentials_database: Default::default(),
            reply_surb_database: self.reply_surb_database.clone(),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        )
    }

    pub fn x25519_diffie_hellman_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_diffie_hellman_key_file,
            &self.public_x25519_diffie_hellman_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayPathsV7 {
    pub clients_storage: PathBuf,

    pub stats_storage: PathBuf,

    pub network_requester: NetworkRequesterPathsV7,

    pub ip_packet_router: IpPacketRouterPathsV7,

    pub authenticator: AuthenticatorPathsV7,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AuthenticatorV7 {
    #[serde(default)]
    pub debug: AuthenticatorDebugV7,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct AuthenticatorDebugV7 {
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

impl Default for AuthenticatorDebugV7 {
    fn default() -> Self {
        AuthenticatorDebugV7 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AuthenticatorV7 {
    fn default() -> Self {
        AuthenticatorV7 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct IpPacketRouterDebugV7 {
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

impl Default for IpPacketRouterDebugV7 {
    fn default() -> Self {
        IpPacketRouterDebugV7 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct IpPacketRouterV7 {
    #[serde(default)]
    pub debug: IpPacketRouterDebugV7,
}

#[allow(clippy::derivable_impls)]
impl Default for IpPacketRouterV7 {
    fn default() -> Self {
        IpPacketRouterV7 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequesterDebugV7 {
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

impl Default for NetworkRequesterDebugV7 {
    fn default() -> Self {
        NetworkRequesterDebugV7 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequesterV7 {
    #[serde(default)]
    pub debug: NetworkRequesterDebugV7,
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequesterV7 {
    fn default() -> Self {
        NetworkRequesterV7 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayDebugV7 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
}

impl ExitGatewayDebugV7 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for ExitGatewayDebugV7 {
    fn default() -> Self {
        ExitGatewayDebugV7 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayConfigV7 {
    pub storage_paths: ExitGatewayPathsV7,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Url,

    pub network_requester: NetworkRequesterV7,

    pub ip_packet_router: IpPacketRouterV7,

    #[serde(default)]
    pub debug: ExitGatewayDebugV7,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksPathsV7 {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys, available client bandwidths and wireguard peers.
    pub clients_storage: PathBuf,

    /// Path to sqlite database containing all persistent stats data.
    pub stats_storage: PathBuf,

    /// Path to file containing cosmos account mnemonic used for zk-nym redemption.
    pub cosmos_mnemonic: PathBuf,
}

impl GatewayTasksPathsV7 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        GatewayTasksPathsV7 {
            clients_storage: data_dir.as_ref().join(DEFAULT_CLIENTS_STORAGE_FILENAME),
            stats_storage: data_dir.as_ref().join(DEFAULT_STATS_STORAGE_FILENAME),
            cosmos_mnemonic: data_dir.as_ref().join(DEFAULT_MNEMONIC_FILENAME),
        }
    }

    pub fn load_mnemonic_from_file(&self) -> Result<Zeroizing<bip39::Mnemonic>, EntryGatewayError> {
        let stringified =
            Zeroizing::new(fs::read_to_string(&self.cosmos_mnemonic).map_err(|source| {
                EntryGatewayError::MnemonicLoadFailure {
                    path: self.cosmos_mnemonic.clone(),
                    source,
                }
            })?);

        Ok(Zeroizing::new(bip39::Mnemonic::parse::<&str>(
            stringified.as_ref(),
        )?))
    }

    pub fn save_mnemonic_to_file(
        &self,
        mnemonic: &bip39::Mnemonic,
    ) -> Result<(), EntryGatewayError> {
        // wrapper for io errors
        fn _save_to_file(path: &Path, mnemonic: &bip39::Mnemonic) -> io::Result<()> {
            if let Some(parent) = path.parent() {
                create_dir_all(parent)?;
            }
            info!("saving entry gateway mnemonic to '{}'", path.display());

            let stringified = Zeroizing::new(mnemonic.to_string());
            fs::write(path, &stringified)
        }

        _save_to_file(&self.cosmos_mnemonic, mnemonic).map_err(|source| {
            EntryGatewayError::MnemonicSaveFailure {
                path: self.cosmos_mnemonic.clone(),
                source,
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StaleMessageDebugV7 {
    /// Specifies how often the clean-up task should check for stale data.
    #[serde(with = "humantime_serde")]
    pub cleaner_run_interval: Duration,

    /// Specifies maximum age of stored messages before they are removed from the storage
    #[serde(with = "humantime_serde")]
    pub max_age: Duration,
}

impl StaleMessageDebugV7 {
    const DEFAULT_STALE_MESSAGES_CLEANER_RUN_INTERVAL: Duration = Duration::from_secs(60 * 60);
    const DEFAULT_STALE_MESSAGES_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
}

impl Default for StaleMessageDebugV7 {
    fn default() -> Self {
        StaleMessageDebugV7 {
            cleaner_run_interval: Self::DEFAULT_STALE_MESSAGES_CLEANER_RUN_INTERVAL,
            max_age: Self::DEFAULT_STALE_MESSAGES_MAX_AGE,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClientBandwidthDebugV7 {
    /// Defines maximum delay between client bandwidth information being flushed to the persistent storage.
    pub max_flushing_rate: Duration,

    /// Defines a maximum change in client bandwidth before it gets flushed to the persistent storage.
    pub max_delta_flushing_amount: i64,
}

impl ClientBandwidthDebugV7 {
    const DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE: Duration = Duration::from_millis(5);
    const DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT: i64 = 512 * 1024; // 512kB
}

impl Default for ClientBandwidthDebugV7 {
    fn default() -> Self {
        ClientBandwidthDebugV7 {
            max_flushing_rate: Self::DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE,
            max_delta_flushing_amount: Self::DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct GatewayTasksConfigDebugV7 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,

    pub stale_messages: StaleMessageDebugV7,

    pub client_bandwidth: ClientBandwidthDebugV7,

    pub zk_nym_tickets: ZkNymTicketHandlerDebugV7,
}

impl GatewayTasksConfigDebugV7 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for GatewayTasksConfigDebugV7 {
    fn default() -> Self {
        GatewayTasksConfigDebugV7 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            stale_messages: Default::default(),
            client_bandwidth: Default::default(),
            zk_nym_tickets: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksConfigV7 {
    pub storage_paths: GatewayTasksPathsV7,

    /// Indicates whether this gateway is accepting only zk-nym credentials for accessing the mixnet
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
    pub debug: GatewayTasksConfigDebugV7,
}

impl GatewayTasksConfigV7 {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        GatewayTasksConfigV7 {
            storage_paths: GatewayTasksPathsV7::new(data_dir),
            enforce_zk_nyms: false,
            bind_address: SocketAddr::new(in6addr_any_init(), DEFAULT_WS_PORT),
            announce_ws_port: None,
            announce_wss_port: None,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvidersPathsV7 {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys, available client bandwidths and wireguard peers.
    pub clients_storage: PathBuf,

    /// Path to sqlite database containing all persistent stats data.
    pub stats_storage: PathBuf,

    pub network_requester: NetworkRequesterPathsV7,

    pub ip_packet_router: IpPacketRouterPathsV7,

    pub authenticator: AuthenticatorPathsV7,
}

impl ServiceProvidersPathsV7 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        ServiceProvidersPathsV7 {
            clients_storage: data_dir.join(DEFAULT_CLIENTS_STORAGE_FILENAME),
            stats_storage: data_dir.join(DEFAULT_STATS_STORAGE_FILENAME),
            network_requester: NetworkRequesterPathsV7::new(data_dir),
            ip_packet_router: IpPacketRouterPathsV7::new(data_dir),
            authenticator: AuthenticatorPathsV7::new(data_dir),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvidersConfigDebugV7 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
}

impl ServiceProvidersConfigDebugV7 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for ServiceProvidersConfigDebugV7 {
    fn default() -> Self {
        ServiceProvidersConfigDebugV7 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvidersConfigV7 {
    pub storage_paths: ServiceProvidersPathsV7,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Url,

    pub network_requester: NetworkRequesterV7,

    pub ip_packet_router: IpPacketRouterV7,

    pub authenticator: AuthenticatorV7,

    #[serde(default)]
    pub debug: ServiceProvidersConfigDebugV7,
}

impl ServiceProvidersConfigV7 {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        #[allow(clippy::expect_used)]
        // SAFETY:
        // we expect our default values to be well-formed
        ServiceProvidersConfigV7 {
            storage_paths: ServiceProvidersPathsV7::new(data_dir),
            open_proxy: false,
            upstream_exit_policy_url: mainnet::EXIT_POLICY_URL
                .parse()
                .expect("invalid default exit policy URL"),
            network_requester: Default::default(),
            ip_packet_router: Default::default(),
            authenticator: Default::default(),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsConfigV7 {
    #[serde(default)]
    pub debug: MetricsDebugV7,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsDebugV7 {
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

impl MetricsDebugV7 {
    const DEFAULT_CONSOLE_LOGGING_INTERVAL: Duration = Duration::from_millis(60_000);
    const DEFAULT_LEGACY_MIXING_UPDATE_RATE: Duration = Duration::from_millis(30_000);
    const DEFAULT_AGGREGATOR_UPDATE_RATE: Duration = Duration::from_secs(5);
    const DEFAULT_STALE_MIXNET_METRICS_UPDATE_RATE: Duration = Duration::from_secs(3600);
    const DEFAULT_CLIENT_SESSIONS_UPDATE_RATE: Duration = Duration::from_secs(3600);
    const GLOBAL_PROMETHEUS_COUNTERS_UPDATE_INTERVAL: Duration = Duration::from_secs(30);
    const DEFAULT_PENDING_EGRESS_PACKETS_UPDATE_RATE: Duration = Duration::from_secs(30);
}

impl Default for MetricsDebugV7 {
    fn default() -> Self {
        MetricsDebugV7 {
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
pub struct LoggingSettingsV7 {
    // well, we need to implement something here at some point...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV7 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModesV7,

    pub host: HostV7,

    pub mixnet: MixnetV7,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePathsV7,

    #[serde(default)]
    pub http: HttpV7,

    #[serde(default)]
    pub verloc: VerlocV7,

    pub wireguard: WireguardV7,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfigV7,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfigV7,

    #[serde(default)]
    pub metrics: MetricsConfigV7,

    #[serde(default)]
    pub logging: LoggingSettingsV7,

    #[serde(default)]
    pub debug: DebugV7,
}

impl ConfigV7 {
    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: ConfigV7 =
            read_config_from_toml_file(path).map_err(|source| NymNodeError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }
}

#[instrument(skip_all)]
pub async fn try_upgrade_config_v7<P: AsRef<Path>>(
    path: P,
    prev_config: Option<ConfigV7>,
) -> Result<Config, NymNodeError> {
    debug!("attempting to load v7 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV7::read_from_path(&path)?
    };

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
            bind_address: {
                if old_cfg.mixnet.bind_address.ip().is_unspecified() {
                    SocketAddr::new(in6addr_any_init(), old_cfg.mixnet.bind_address.port())
                } else {
                    old_cfg.mixnet.bind_address
                }
            },
            announce_port: old_cfg.mixnet.announce_port,
            nym_api_urls: old_cfg.mixnet.nym_api_urls,
            nyxd_urls: old_cfg.mixnet.nyxd_urls,
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
                private_x25519_sphinx_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_x25519_sphinx_key_file,
                public_x25519_sphinx_key_file: old_cfg
                    .storage_paths
                    .keys
                    .public_x25519_sphinx_key_file,
                private_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_x25519_noise_key_file,
                public_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .public_x25519_noise_key_file,
            },
            description: old_cfg.storage_paths.description,
        },
        http: Http {
            bind_address: {
                if old_cfg.http.bind_address.ip().is_unspecified() {
                    SocketAddr::new(in6addr_any_init(), old_cfg.http.bind_address.port())
                } else {
                    old_cfg.http.bind_address
                }
            },
            landing_page_assets_path: old_cfg.http.landing_page_assets_path,
            access_token: old_cfg.http.access_token,
            expose_system_info: old_cfg.http.expose_system_info,
            expose_system_hardware: old_cfg.http.expose_system_hardware,
            expose_crypto_hardware: old_cfg.http.expose_crypto_hardware,
        },
        verloc: Verloc {
            bind_address: {
                if old_cfg.verloc.bind_address.ip().is_unspecified() {
                    SocketAddr::new(in6addr_any_init(), old_cfg.verloc.bind_address.port())
                } else {
                    old_cfg.verloc.bind_address
                }
            },
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
            bind_address: {
                if old_cfg.wireguard.bind_address.ip().is_unspecified() {
                    SocketAddr::new(in6addr_any_init(), old_cfg.wireguard.bind_address.port())
                } else {
                    old_cfg.wireguard.bind_address
                }
            },
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
            ws_bind_address: {
                if old_cfg.gateway_tasks.bind_address.ip().is_unspecified() {
                    SocketAddr::new(
                        in6addr_any_init(),
                        old_cfg.gateway_tasks.bind_address.port(),
                    )
                } else {
                    old_cfg.gateway_tasks.bind_address
                }
            },
            announce_ws_port: old_cfg.gateway_tasks.announce_ws_port,
            announce_wss_port: old_cfg.gateway_tasks.announce_wss_port,
            debug: gateway_tasks::Debug {
                message_retrieval_limit: old_cfg.gateway_tasks.debug.message_retrieval_limit,
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
                ..Default::default()
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
