// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![allow(dead_code)]
use crate::config::*;
use crate::error::KeyIOFailure;
use nym_client_core_config_types::DebugConfig as ClientDebugConfig;
use nym_config::serde_helpers::de_maybe_port;
use nym_crypto::asymmetric::encryption::KeyPair;
use nym_pemstore::store_keypair;
use old_configs::old_config_1_1_3::*;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardPaths1_1_2 {
    pub private_diffie_hellman_key_file: PathBuf,
    pub public_diffie_hellman_key_file: PathBuf,
}

impl WireguardPaths1_1_2 {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        WireguardPaths1_1_2 {
            private_diffie_hellman_key_file: data_dir
                .join(persistence::DEFAULT_X25519_WG_DH_KEY_FILENAME),
            public_diffie_hellman_key_file: data_dir
                .join(persistence::DEFAULT_X25519_WG_PUBLIC_DH_KEY_FILENAME),
        }
    }

    pub fn x25519_wireguard_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_diffie_hellman_key_file,
            &self.public_diffie_hellman_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wireguard1_1_2 {
    /// Specifies whether the wireguard service is enabled on this node.
    pub enabled: bool,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    pub bind_address: SocketAddr,

    /// Ip address of the private wireguard network.
    /// default: `10.1.0.0`
    pub private_network_ip: IpAddr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard.
    /// The maximum value for IPv4 is 32 and for IPv6 is 128
    pub private_network_prefix: u8,

    /// Paths for wireguard keys, client registries, etc.
    pub storage_paths: WireguardPaths1_1_2,
}

// a temporary solution until all "types" are run at the same time
#[derive(Debug, Default, Serialize, Deserialize, ValueEnum, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum NodeMode1_1_2 {
    #[default]
    #[clap(alias = "mix")]
    Mixnode,

    #[clap(alias = "entry", alias = "gateway")]
    EntryGateway,

    #[clap(alias = "exit")]
    ExitGateway,
}

impl From<NodeMode1_1_2> for NodeMode1_1_3 {
    fn from(config: NodeMode1_1_2) -> Self {
        match config {
            NodeMode1_1_2::Mixnode => NodeMode1_1_3::Mixnode,
            NodeMode1_1_2::EntryGateway => NodeMode1_1_3::EntryGateway,
            NodeMode1_1_2::ExitGateway => NodeMode1_1_3::ExitGateway,
        }
    }
}

// TODO: this is very much a WIP. we need proper ssl certificate support here
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Host1_1_2 {
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
pub struct MixnetDebug1_1_2 {
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

impl MixnetDebug1_1_2 {
    const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
    const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
    const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
    const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;
}

impl Default for MixnetDebug1_1_2 {
    fn default() -> Self {
        MixnetDebug1_1_2 {
            packet_forwarding_initial_backoff: Self::DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: Self::DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: Self::DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: Self::DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            // to be changed by @SW once the implementation is there
            unsafe_disable_noise: true,
        }
    }
}

impl Default for Mixnet1_1_2 {
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

        Mixnet1_1_2 {
            bind_address: SocketAddr::new(inaddr_any(), DEFAULT_MIXNET_PORT),
            nym_api_urls,
            nyxd_urls,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Mixnet1_1_2 {
    /// Address this node will bind to for listening for mixnet packets
    /// default: `0.0.0.0:1789`
    pub bind_address: SocketAddr,

    /// Addresses to nym APIs from which the node gets the view of the network.
    pub nym_api_urls: Vec<Url>,

    /// Addresses to nyxd which the node uses to interact with the nyx chain.
    pub nyxd_urls: Vec<Url>,

    #[serde(default)]
    pub debug: MixnetDebug1_1_2,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeysPaths1_1_2 {
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
pub struct NymNodePaths1_1_2 {
    pub keys: KeysPaths1_1_2,

    /// Path to a file containing basic node description: human-readable name, website, details, etc.
    pub description: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Http1_1_2 {
    /// Socket address this node will use for binding its http API.
    /// default: `0.0.0.0:8080`
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

impl Default for Http1_1_2 {
    fn default() -> Self {
        Http1_1_2 {
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
pub struct MixnodePaths1_1_2 {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debug1_1_2 {
    /// Delay between each subsequent node statistics being logged to the console
    #[serde(with = "humantime_serde")]
    pub node_stats_logging_delay: Duration,

    /// Delay between each subsequent node statistics being updated
    #[serde(with = "humantime_serde")]
    pub node_stats_updating_delay: Duration,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerlocDebug1_1_2 {
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
pub struct Verloc1_1_2 {
    /// Socket address this node will use for binding its verloc API.
    /// default: `0.0.0.0:1790`
    pub bind_address: SocketAddr,

    #[serde(default)]
    pub debug: VerlocDebug1_1_2,
}

impl VerlocDebug1_1_2 {
    const DEFAULT_PACKETS_PER_NODE: usize = 100;
    const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
    const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
    const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
    const DEFAULT_BATCH_SIZE: usize = 50;
    const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
    const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);
}

impl Default for VerlocDebug1_1_2 {
    fn default() -> Self {
        VerlocDebug1_1_2 {
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
pub struct MixnodeConfig1_1_2 {
    pub storage_paths: MixnodePaths1_1_2,

    pub verloc: Verloc1_1_2,

    #[serde(default)]
    pub debug: Debug1_1_2,
}

impl Debug1_1_2 {
    const DEFAULT_NODE_STATS_LOGGING_DELAY: Duration = Duration::from_millis(60_000);
    const DEFAULT_NODE_STATS_UPDATING_DELAY: Duration = Duration::from_millis(30_000);
}

impl Default for Debug1_1_2 {
    fn default() -> Self {
        Debug1_1_2 {
            node_stats_logging_delay: Self::DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: Self::DEFAULT_NODE_STATS_UPDATING_DELAY,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayPaths1_1_2 {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys and available client bandwidths.
    pub clients_storage: PathBuf,

    /// Path to file containing cosmos account mnemonic used for zk-nym redemption.
    pub cosmos_mnemonic: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayConfigDebug1_1_2 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
}

impl EntryGatewayConfigDebug1_1_2 {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for EntryGatewayConfigDebug1_1_2 {
    fn default() -> Self {
        EntryGatewayConfigDebug1_1_2 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayConfig1_1_2 {
    pub storage_paths: EntryGatewayPaths1_1_2,

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
    pub debug: EntryGatewayConfigDebug1_1_2,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkRequesterPaths1_1_2 {
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
pub struct IpPacketRouterPaths1_1_2 {
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
pub struct ExitGatewayPaths1_1_2 {
    pub network_requester: NetworkRequesterPaths1_1_2,

    pub ip_packet_router: IpPacketRouterPaths1_1_2,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct IpPacketRouterDebug1_1_2 {
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

impl Default for IpPacketRouterDebug1_1_2 {
    fn default() -> Self {
        IpPacketRouterDebug1_1_2 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct IpPacketRouter1_1_2 {
    #[serde(default)]
    pub debug: IpPacketRouterDebug1_1_2,
}

#[allow(clippy::derivable_impls)]
impl Default for IpPacketRouter1_1_2 {
    fn default() -> Self {
        IpPacketRouter1_1_2 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequesterDebug1_1_2 {
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

impl Default for NetworkRequesterDebug1_1_2 {
    fn default() -> Self {
        NetworkRequesterDebug1_1_2 {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequester1_1_2 {
    #[serde(default)]
    pub debug: NetworkRequesterDebug1_1_2,
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequester1_1_2 {
    fn default() -> Self {
        NetworkRequester1_1_2 {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayConfig1_1_2 {
    pub storage_paths: ExitGatewayPaths1_1_2,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Url,

    pub network_requester: NetworkRequester1_1_2,

    pub ip_packet_router: IpPacketRouter1_1_2,
}

#[derive(Debug, Default, Copy, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettings1_1_2 {
    // well, we need to implement something here at some point...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config1_1_2 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current mode of this nym-node.
    /// Expect this field to be changed in the future to allow running the node in multiple modes (i.e. mixnode + gateway)
    pub mode: NodeMode1_1_2,

    pub host: Host1_1_2,

    pub mixnet: Mixnet1_1_2,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePaths1_1_2,

    #[serde(default)]
    pub http: Http1_1_2,

    pub wireguard: Wireguard1_1_2,

    pub mixnode: MixnodeConfig1_1_2,

    pub entry_gateway: EntryGatewayConfig1_1_2,

    pub exit_gateway: ExitGatewayConfig1_1_2,

    #[serde(default)]
    pub logging: LoggingSettings1_1_2,
}

impl NymConfigTemplate for Config1_1_2 {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config1_1_2 {
    pub fn save(&self) -> Result<(), NymNodeError> {
        let save_location = self.save_location();
        debug!(
            "attempting to save config file to '{}'",
            save_location.display()
        );
        save_formatted_config_to_file(self, &save_location).map_err(|source| {
            NymNodeError::ConfigSaveFailure {
                id: self.id.clone(),
                path: save_location,
                source,
            }
        })
    }

    pub fn save_location(&self) -> PathBuf {
        self.save_path
            .clone()
            .unwrap_or(self.default_save_location())
    }

    pub fn default_save_location(&self) -> PathBuf {
        default_config_filepath(&self.id)
    }

    pub fn default_data_directory<P: AsRef<Path>>(config_path: P) -> Result<PathBuf, NymNodeError> {
        let config_path = config_path.as_ref();

        // we got a proper path to the .toml file
        let Some(config_dir) = config_path.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_path.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        let Some(config_dir_name) = config_dir.file_name() else {
            error!(
                "could not obtain parent directory name of '{}'. Have you used relative paths?",
                config_path.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        if config_dir_name != DEFAULT_CONFIG_DIR {
            error!(
                "the parent directory of '{}' ({}) is not {DEFAULT_CONFIG_DIR}. currently this is not supported",
                config_path.display(), config_dir_name.to_str().unwrap_or("UNKNOWN")
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        }

        let Some(node_dir) = config_dir.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_dir.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        Ok(node_dir.join(DEFAULT_DATA_DIR))
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: Config1_1_2 =
            read_config_from_toml_file(path).map_err(|source| NymNodeError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        Self::read_from_path(path)
    }
}

fn initialise(config: &Wireguard1_1_3) -> std::io::Result<()> {
    let mut rng = OsRng;
    let x25519_keys = KeyPair::new(&mut rng);

    store_keypair(
        &x25519_keys,
        &config.storage_paths.x25519_wireguard_storage_paths(),
    )?;

    Ok(())
}

// currently there are no upgrades
pub async fn try_upgrade_config_1_1_2<P: AsRef<Path>>(path: P) -> Result<(), NymNodeError> {
    let old_cfg = Config1_1_2::read_from_path(&path)?;
    let wireguard = Wireguard1_1_3 {
        enabled: old_cfg.wireguard.enabled,
        bind_address: old_cfg.wireguard.bind_address,
        private_ip: old_cfg.wireguard.private_network_ip,
        announced_port: old_cfg.wireguard.announced_port,
        private_network_prefix: old_cfg.wireguard.private_network_prefix,
        storage_paths: WireguardPaths1_1_3::new(Config1_1_3::default_data_directory(path)?),
    };
    initialise(&wireguard).map_err(|err| KeyIOFailure::KeyPairStoreFailure {
        keys: "wg-x25519-dh".to_string(),
        paths: wireguard.storage_paths.x25519_wireguard_storage_paths(),
        err,
    })?;
    let cfg = Config1_1_3 {
        save_path: old_cfg.save_path,
        id: old_cfg.id,
        mode: old_cfg.mode.into(),
        host: Host1_1_3 {
            public_ips: old_cfg.host.public_ips,
            hostname: old_cfg.host.hostname,
            location: old_cfg.host.location,
        },
        mixnet: Mixnet1_1_3 {
            bind_address: old_cfg.mixnet.bind_address,
            nym_api_urls: old_cfg.mixnet.nym_api_urls,
            nyxd_urls: old_cfg.mixnet.nyxd_urls,
            debug: MixnetDebug1_1_3 {
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
        storage_paths: NymNodePaths1_1_3 {
            keys: KeysPaths1_1_3 {
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
        http: Http1_1_3 {
            bind_address: old_cfg.http.bind_address,
            landing_page_assets_path: old_cfg.http.landing_page_assets_path,
            access_token: old_cfg.http.access_token,
            expose_system_info: old_cfg.http.expose_system_info,
            expose_system_hardware: old_cfg.http.expose_system_hardware,
            expose_crypto_hardware: old_cfg.http.expose_crypto_hardware,
        },
        wireguard,
        mixnode: MixnodeConfig1_1_3 {
            storage_paths: MixnodePaths1_1_3 {},
            verloc: Verloc1_1_3 {
                bind_address: old_cfg.mixnode.verloc.bind_address,
                debug: VerlocDebug1_1_3 {
                    packets_per_node: old_cfg.mixnode.verloc.debug.packets_per_node,
                    connection_timeout: old_cfg.mixnode.verloc.debug.connection_timeout,
                    packet_timeout: old_cfg.mixnode.verloc.debug.packet_timeout,
                    delay_between_packets: old_cfg.mixnode.verloc.debug.delay_between_packets,
                    tested_nodes_batch_size: old_cfg.mixnode.verloc.debug.tested_nodes_batch_size,
                    testing_interval: old_cfg.mixnode.verloc.debug.testing_interval,
                    retry_timeout: old_cfg.mixnode.verloc.debug.retry_timeout,
                },
            },
            debug: Debug1_1_3 {
                node_stats_logging_delay: old_cfg.mixnode.debug.node_stats_logging_delay,
                node_stats_updating_delay: old_cfg.mixnode.debug.node_stats_updating_delay,
            },
        },
        entry_gateway: EntryGatewayConfig1_1_3 {
            storage_paths: EntryGatewayPaths1_1_3 {
                clients_storage: old_cfg.entry_gateway.storage_paths.clients_storage,
                cosmos_mnemonic: old_cfg.entry_gateway.storage_paths.cosmos_mnemonic,
            },
            enforce_zk_nyms: old_cfg.entry_gateway.enforce_zk_nyms,
            bind_address: old_cfg.entry_gateway.bind_address,
            announce_ws_port: old_cfg.entry_gateway.announce_ws_port,
            announce_wss_port: old_cfg.entry_gateway.announce_wss_port,
            debug: EntryGatewayConfigDebug1_1_3 {
                message_retrieval_limit: old_cfg.entry_gateway.debug.message_retrieval_limit,
            },
        },
        exit_gateway: ExitGatewayConfig1_1_3 {
            storage_paths: ExitGatewayPaths1_1_3 {
                network_requester: NetworkRequesterPaths1_1_3 {
                    private_ed25519_identity_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .network_requester
                        .private_ed25519_identity_key_file,
                    public_ed25519_identity_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .network_requester
                        .public_ed25519_identity_key_file,
                    private_x25519_diffie_hellman_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .network_requester
                        .private_x25519_diffie_hellman_key_file,
                    public_x25519_diffie_hellman_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .network_requester
                        .public_x25519_diffie_hellman_key_file,
                    ack_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .network_requester
                        .ack_key_file,
                    reply_surb_database: old_cfg
                        .exit_gateway
                        .storage_paths
                        .network_requester
                        .reply_surb_database,
                    gateway_registrations: old_cfg
                        .exit_gateway
                        .storage_paths
                        .network_requester
                        .gateway_registrations,
                },
                ip_packet_router: IpPacketRouterPaths1_1_3 {
                    private_ed25519_identity_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .ip_packet_router
                        .private_ed25519_identity_key_file,
                    public_ed25519_identity_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .ip_packet_router
                        .public_ed25519_identity_key_file,
                    private_x25519_diffie_hellman_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .ip_packet_router
                        .private_x25519_diffie_hellman_key_file,
                    public_x25519_diffie_hellman_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .ip_packet_router
                        .public_x25519_diffie_hellman_key_file,
                    ack_key_file: old_cfg
                        .exit_gateway
                        .storage_paths
                        .ip_packet_router
                        .ack_key_file,
                    reply_surb_database: old_cfg
                        .exit_gateway
                        .storage_paths
                        .ip_packet_router
                        .reply_surb_database,
                    gateway_registrations: old_cfg
                        .exit_gateway
                        .storage_paths
                        .ip_packet_router
                        .gateway_registrations,
                },
            },
            open_proxy: old_cfg.exit_gateway.open_proxy,
            upstream_exit_policy_url: old_cfg.exit_gateway.upstream_exit_policy_url,
            network_requester: NetworkRequester1_1_3 {
                debug: NetworkRequesterDebug1_1_3 {
                    enabled: old_cfg.exit_gateway.network_requester.debug.enabled,
                    disable_poisson_rate: old_cfg
                        .exit_gateway
                        .network_requester
                        .debug
                        .disable_poisson_rate,
                    client_debug: old_cfg.exit_gateway.network_requester.debug.client_debug,
                },
            },
            ip_packet_router: IpPacketRouter1_1_3 {
                debug: IpPacketRouterDebug1_1_3 {
                    enabled: old_cfg.exit_gateway.ip_packet_router.debug.enabled,
                    disable_poisson_rate: old_cfg
                        .exit_gateway
                        .ip_packet_router
                        .debug
                        .disable_poisson_rate,
                    client_debug: old_cfg.exit_gateway.ip_packet_router.debug.client_debug,
                },
            },
        },
        logging: LoggingSettings1_1_3 {},
    };

    cfg.save()?;

    Ok(())
}
