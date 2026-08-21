// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::old_configs::old_config_v13::unchanged_v13_types::{
    ClientBandwidthDebugV13, DebugV13, GatewayTasksPathsV13, HostV13, HttpV13, LoggingSettingsV13,
    MetricsConfigV13, MixnetV13, NodeModesV13, ServiceProvidersConfigV13, StaleMessageDebugV13,
    UpgradeModeWatcherV13, VerlocV13, WireguardV13, ZkNymTicketHandlerDebugV13,
};
use crate::config::old_configs::old_config_v14::{ConfigV14, NyxV14};
use crate::error::NymNodeError;
use nym_config::read_config_from_toml_file;
use nym_config::serde_helpers::de_maybe_port;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info, instrument};

#[allow(unused_imports)]
pub use unchanged_v13_types::*;

// (while some of those are technically unused, they might be needed in future migrations,
// thus allow them to exist)
#[allow(dead_code)]
pub mod unchanged_v13_types {
    use crate::config::old_configs::old_config_v12::{
        AuthenticatorDebugV12, AuthenticatorPathsV12, AuthenticatorV12, ClientBandwidthDebugV12,
        DebugV12, GatewayTasksPathsV12, HostV12, HttpV12, IpPacketRouterDebugV12,
        IpPacketRouterPathsV12, IpPacketRouterV12, KeyRotationDebugV12, KeyRotationV12,
        KeysPathsV12, LoggingSettingsV12, MetricsConfigV12, MetricsDebugV12, MixnetDebugV12,
        MixnetV12, NetworkRequesterDebugV12, NetworkRequesterPathsV12, NetworkRequesterV12,
        NodeModeV12, NodeModesV12, ReplayProtectionDebugV12, ReplayProtectionPathsV12,
        ReplayProtectionV12, ServiceProvidersConfigDebugV12, ServiceProvidersConfigV12,
        ServiceProvidersPathsV12, StaleMessageDebugV12, UpgradeModeWatcherV12, VerlocDebugV12,
        VerlocV12, WireguardPathsV12, WireguardV12, ZkNymTicketHandlerDebugV12,
    };

    pub type WireguardPathsV13 = WireguardPathsV12;
    pub type NodeModeV13 = NodeModeV12;
    pub type NodeModesV13 = NodeModesV12;
    pub type HostV13 = HostV12;
    pub type KeyRotationDebugV13 = KeyRotationDebugV12;
    pub type KeyRotationV13 = KeyRotationV12;
    pub type MixnetDebugV13 = MixnetDebugV12;
    pub type MixnetV13 = MixnetV12;
    pub type ReplayProtectionV13 = ReplayProtectionV12;
    pub type ReplayProtectionPathsV13 = ReplayProtectionPathsV12;
    pub type ReplayProtectionDebugV13 = ReplayProtectionDebugV12;
    pub type KeysPathsV13 = KeysPathsV12;
    pub type HttpV13 = HttpV12;
    pub type VerlocDebugV13 = VerlocDebugV12;
    pub type VerlocV13 = VerlocV12;
    pub type ZkNymTicketHandlerDebugV13 = ZkNymTicketHandlerDebugV12;
    pub type NetworkRequesterPathsV13 = NetworkRequesterPathsV12;
    pub type IpPacketRouterPathsV13 = IpPacketRouterPathsV12;
    pub type AuthenticatorPathsV13 = AuthenticatorPathsV12;
    pub type AuthenticatorV13 = AuthenticatorV12;
    pub type AuthenticatorDebugV13 = AuthenticatorDebugV12;
    pub type IpPacketRouterDebugV13 = IpPacketRouterDebugV12;
    pub type IpPacketRouterV13 = IpPacketRouterV12;
    pub type NetworkRequesterDebugV13 = NetworkRequesterDebugV12;
    pub type NetworkRequesterV13 = NetworkRequesterV12;
    pub type GatewayTasksPathsV13 = GatewayTasksPathsV12;
    pub type StaleMessageDebugV13 = StaleMessageDebugV12;
    pub type ClientBandwidthDebugV13 = ClientBandwidthDebugV12;
    pub type ServiceProvidersPathsV13 = ServiceProvidersPathsV12;
    pub type ServiceProvidersConfigDebugV13 = ServiceProvidersConfigDebugV12;
    pub type ServiceProvidersConfigV13 = ServiceProvidersConfigV12;
    pub type MetricsConfigV13 = MetricsConfigV12;
    pub type MetricsDebugV13 = MetricsDebugV12;
    pub type LoggingSettingsV13 = LoggingSettingsV12;
    pub type WireguardV13 = WireguardV12;
    pub type DebugV13 = DebugV12;
    pub type UpgradeModeWatcherV13 = UpgradeModeWatcherV12;
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpConfigV13 {
    /// Bind address for the TCP LP control traffic.
    /// default: `[::]:41264`
    pub control_bind_address: SocketAddr,

    /// Bind address for the UDP LP data traffic.
    /// default: `[::]:51264`
    pub data_bind_address: SocketAddr,

    /// Custom announced port for listening for the TCP LP control traffic.
    /// If unspecified, the value from the `control_bind_address` will be used instead
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_control_port: Option<u16>,

    /// Custom announced port for listening for the UDP LP data traffic.
    /// If unspecified, the value from the `data_bind_address` will be used instead
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_data_port: Option<u16>,

    /// Auxiliary configuration
    #[serde(default)]
    pub debug: LpDebugV13,
}

impl Default for LpConfigV13 {
    fn default() -> Self {
        LpConfigV13 {
            control_bind_address: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 41264),
            data_bind_address: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 51264),
            announce_control_port: None,
            announce_data_port: None,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpDebugV13 {
    /// Maximum concurrent connections
    pub max_connections: usize,

    /// Use mock ecash manager for testing (default: false)
    ///
    /// When enabled, the LP listener will use a mock ecash verifier that
    /// accepts any credential without blockchain verification. This is
    /// useful for testing the LP protocol implementation without requiring
    /// a full blockchain/contract setup.
    ///
    /// WARNING: Only use this for local testing! Never enable in production.
    pub use_mock_ecash: bool,

    /// Maximum age of in-progress handshakes before cleanup (default: 90s)
    ///
    /// Handshakes should complete quickly (3-5 packets). This TTL accounts for:
    /// - Network latency and retransmits
    /// - Slow clients
    /// - Clock skew tolerance
    ///
    /// Stale handshakes are removed by the cleanup task to prevent memory leaks.
    #[serde(with = "humantime_serde")]
    pub handshake_ttl: Duration,

    /// Maximum age of established sessions before cleanup (default: 24h)
    ///
    /// Sessions can be long-lived for dVPN tunnels. This TTL should be set
    /// high enough to accommodate expected usage patterns:
    /// - dVPN sessions: hours to days
    /// - Registration: minutes
    ///
    /// Sessions with no activity for this duration are removed by the cleanup task.
    #[serde(with = "humantime_serde")]
    pub session_ttl: Duration,

    /// How often to run the state cleanup task (default: 5 minutes)
    ///
    /// The cleanup task scans for and removes stale handshakes and sessions.
    /// Lower values = more frequent cleanup but higher overhead.
    /// Higher values = less overhead but slower memory reclamation.
    #[serde(with = "humantime_serde")]
    pub state_cleanup_interval: Duration,

    /// Maximum concurrent forward connections (default: 1000)
    ///
    /// Limits simultaneous outbound connections when forwarding LP packets to other gateways
    /// during telescope setup. This prevents file descriptor exhaustion under high load.
    ///
    /// When at capacity, new forward requests return an error, signaling the client
    /// to choose a different gateway.
    pub max_concurrent_forwards: usize,
}

impl LpDebugV13 {
    pub const DEFAULT_MAX_CONNECTIONS: usize = 10000;
    pub const DEFAULT_HANDSHAKE_TTL: Duration = Duration::from_secs(90);
    pub const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(86400);
    pub const DEFAULT_STATE_CLEANUP_INTERVAL: Duration = Duration::from_secs(300);
    pub const DEFAULT_MAX_CONCURRENT_FORWARDS: usize = 1000;
}

impl Default for LpDebugV13 {
    fn default() -> Self {
        LpDebugV13 {
            max_connections: Self::DEFAULT_MAX_CONNECTIONS,
            use_mock_ecash: false,
            handshake_ttl: Self::DEFAULT_HANDSHAKE_TTL,
            session_ttl: Self::DEFAULT_SESSION_TTL,
            state_cleanup_interval: Self::DEFAULT_STATE_CLEANUP_INTERVAL,
            max_concurrent_forwards: Self::DEFAULT_MAX_CONCURRENT_FORWARDS,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct KeysPathsV13 {
    /// Path to file containing ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing the primary x25519 sphinx private key.
    pub primary_x25519_sphinx_key_file: PathBuf,

    /// Path to file containing the secondary x25519 sphinx private key.
    pub secondary_x25519_sphinx_key_file: PathBuf,

    /// Path to file containing x25519 noise private key.
    pub private_x25519_noise_key_file: PathBuf,

    /// Path to file containing x25519 noise public key.
    pub public_x25519_noise_key_file: PathBuf,

    // >> LP KEYS START:
    /// Path to file containing x25519 lp private key.
    pub private_x25519_lp_key_file: PathBuf,

    /// Path to file containing x25519 lp public key.
    pub public_x25519_lp_key_file: PathBuf,

    /// Path to file containing mlkem768 lp private key.
    pub private_mlkem768_lp_key_file: PathBuf,

    /// Path to file containing mlkem768 lp public key.
    pub public_mlkem768_lp_key_file: PathBuf,

    /// Path to file containing mceliece lp private key.
    pub private_mceliece_lp_key_file: PathBuf,

    /// Path to file containing mceliece lp public key.
    pub public_mceliece_lp_key_file: PathBuf,
    // >> LP KEYS END
}

impl KeysPathsV13 {
    pub fn x25519_lp_key_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_lp_key_file,
            &self.public_x25519_lp_key_file,
        )
    }

    pub fn mlkem768_key_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_mlkem768_lp_key_file,
            &self.public_mlkem768_lp_key_file,
        )
    }

    pub fn mceliece_key_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_mceliece_lp_key_file,
            &self.public_mceliece_lp_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NymNodePathsV13 {
    pub keys: KeysPathsV13,
    pub description: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct GatewayTasksConfigDebugV13 {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,

    /// The maximum number of client connections the gateway will keep open at once.
    pub maximum_open_connections: usize,

    /// Specifies the minimum performance of mixnodes in the network that are to be used in internal topologies
    /// of the services providers
    pub minimum_mix_performance: u8,

    /// Specifies the maximum time this node will wait for its initial valid topology
    #[serde(with = "humantime_serde")]
    pub maximum_initial_topology_waiting_time: Duration,

    /// Defines the timestamp skew of a signed authentication request before it's deemed too excessive to process.
    #[serde(alias = "maximum_auth_request_age")]
    pub max_request_timestamp_skew: Duration,

    /// The minimum duration since the last explicit check for the upgrade mode to allow creation of new requests.
    #[serde(with = "humantime_serde")]
    pub upgrade_mode_min_staleness_recheck: Duration,

    pub stale_messages: StaleMessageDebugV13,

    pub client_bandwidth: ClientBandwidthDebugV13,

    pub zk_nym_tickets: ZkNymTicketHandlerDebugV13,
}

impl GatewayTasksConfigDebugV13 {
    pub const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
    pub const DEFAULT_MINIMUM_MIX_PERFORMANCE: u8 = 50;
    pub const DEFAULT_MAXIMUM_AUTH_REQUEST_TIMESTAMP_SKEW: Duration = Duration::from_secs(120);
    pub const DEFAULT_MAXIMUM_OPEN_CONNECTIONS: usize = 8192;
    pub const DEFAULT_UPGRADE_MODE_MIN_STALENESS_RECHECK: Duration = Duration::from_secs(30);
    pub const DEFAULT_MAXIMUM_INITIAL_TOPOLOGY_WAITING_TIME: Duration =
        Duration::from_secs(15 * 60);
}

impl Default for GatewayTasksConfigDebugV13 {
    fn default() -> Self {
        GatewayTasksConfigDebugV13 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            maximum_open_connections: Self::DEFAULT_MAXIMUM_OPEN_CONNECTIONS,
            max_request_timestamp_skew: Self::DEFAULT_MAXIMUM_AUTH_REQUEST_TIMESTAMP_SKEW,
            minimum_mix_performance: Self::DEFAULT_MINIMUM_MIX_PERFORMANCE,
            stale_messages: Default::default(),
            client_bandwidth: Default::default(),
            zk_nym_tickets: Default::default(),
            upgrade_mode_min_staleness_recheck: Self::DEFAULT_UPGRADE_MODE_MIN_STALENESS_RECHECK,
            maximum_initial_topology_waiting_time:
                Self::DEFAULT_MAXIMUM_INITIAL_TOPOLOGY_WAITING_TIME,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksConfigV13 {
    pub storage_paths: GatewayTasksPathsV13,

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

    pub upgrade_mode: UpgradeModeWatcherV13,

    #[serde(default)]
    pub debug: GatewayTasksConfigDebugV13,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV13 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModesV13,

    pub host: HostV13,

    pub mixnet: MixnetV13,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePathsV13,

    #[serde(default)]
    pub http: HttpV13,

    #[serde(default)]
    pub verloc: VerlocV13,

    pub wireguard: WireguardV13,

    #[serde(default)]
    pub lp: LpConfigV13,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfigV13,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfigV13,

    #[serde(default)]
    pub metrics: MetricsConfigV13,

    #[serde(default)]
    pub logging: LoggingSettingsV13,

    #[serde(default)]
    pub debug: DebugV13,
}

impl ConfigV13 {
    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: ConfigV13 =
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
pub async fn try_upgrade_config_v13<P: AsRef<Path>>(
    path: P,
    prev_config: Option<ConfigV13>,
) -> Result<ConfigV14, NymNodeError> {
    debug!("attempting to load v13 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV13::read_from_path(&path)?
    };

    info!("migrating the old config (v13)...");

    let cfg = ConfigV14 {
        save_path: old_cfg.save_path,
        id: old_cfg.id,
        modes: old_cfg.modes,
        host: old_cfg.host,
        // \/ ADDED
        // use default to switch to the lite query node
        nyx: NyxV14::default(),
        // /\ ADDED
        mixnet: old_cfg.mixnet,
        storage_paths: old_cfg.storage_paths,
        http: old_cfg.http,
        verloc: old_cfg.verloc,
        wireguard: old_cfg.wireguard,
        lp: old_cfg.lp,
        gateway_tasks: old_cfg.gateway_tasks,
        service_providers: old_cfg.service_providers,
        metrics: old_cfg.metrics,
        logging: old_cfg.logging,
        debug: old_cfg.debug,
    };
    Ok(cfg)
}
