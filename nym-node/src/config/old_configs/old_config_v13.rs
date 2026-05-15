// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::authenticator::{Authenticator, AuthenticatorDebug};
use crate::config::gateway_tasks::{
    ClientBandwidthDebug, StaleMessageDebug, UpgradeModeWatcher, UpgradeModeWatcherDebug,
    ZkNymTicketHandlerDebug,
};
use crate::config::old_configs::old_config_v13::unchanged_v13_types::{
    ClientBandwidthDebugV13, DebugV13, GatewayTasksPathsV13, HostV13, HttpV13, LoggingSettingsV13,
    MetricsConfigV13, MixnetV13, NodeModesV13, ServiceProvidersConfigV13, StaleMessageDebugV13,
    UpgradeModeWatcherV13, VerlocV13, WireguardV13, ZkNymTicketHandlerDebugV13,
};
use crate::config::persistence::{
    AuthenticatorPaths, GatewayTasksPaths, IpPacketRouterPaths, KeysPaths, NetworkRequesterPaths,
    NymNodePaths, ReplayProtectionPaths, ServiceProvidersPaths, WireguardPaths,
};
use crate::config::service_providers::{
    IpPacketRouter, IpPacketRouterDebug, NetworkRequester, NetworkRequesterDebug,
};
use crate::config::{
    Config, Debug, GatewayTasksConfig, Host, Http, KeyRotation, KeyRotationDebug, LpConfig,
    LpDebug, MetricsConfig, Mixnet, MixnetDebug, NodeModes, Nyx, ReplayProtection,
    ReplayProtectionDebug, ServiceProvidersConfig, Verloc, VerlocDebug, Wireguard, gateway_tasks,
    metrics, service_providers,
};
use crate::error::NymNodeError;
use nym_bin_common::logging::LoggingSettings;
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
) -> Result<Config, NymNodeError> {
    debug!("attempting to load v13 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV13::read_from_path(&path)?
    };

    info!("migrating the old config (v13)...");

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
        // \/ ADDED
        nyx: Nyx {
            nyxd_websocket_url: Nyx::default().nyxd_websocket_url,
            nyxd_urls: old_cfg.mixnet.nyxd_urls,
        },
        // /\ ADDED
        mixnet: Mixnet {
            bind_address: old_cfg.mixnet.bind_address,
            announce_port: old_cfg.mixnet.announce_port,
            nym_api_urls: old_cfg.mixnet.nym_api_urls,
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
            key_rotation: KeyRotation {
                debug: KeyRotationDebug {
                    rotation_state_poling_interval: old_cfg
                        .mixnet
                        .key_rotation
                        .debug
                        .rotation_state_poling_interval,
                },
            },
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
                use_legacy_packet_encoding: old_cfg.mixnet.debug.use_legacy_packet_encoding,
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
                primary_x25519_sphinx_key_file: old_cfg
                    .storage_paths
                    .keys
                    .primary_x25519_sphinx_key_file,
                secondary_x25519_sphinx_key_file: old_cfg
                    .storage_paths
                    .keys
                    .secondary_x25519_sphinx_key_file,
                private_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_x25519_noise_key_file,
                public_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .public_x25519_noise_key_file,
                private_x25519_lp_key_file: old_cfg.storage_paths.keys.private_x25519_lp_key_file,
                public_x25519_lp_key_file: old_cfg.storage_paths.keys.public_x25519_lp_key_file,
                private_mlkem768_lp_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_mlkem768_lp_key_file,
                public_mlkem768_lp_key_file: old_cfg.storage_paths.keys.public_mlkem768_lp_key_file,
                private_mceliece_lp_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_mceliece_lp_key_file,
                public_mceliece_lp_key_file: old_cfg.storage_paths.keys.public_mceliece_lp_key_file,
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
            announced_tunnel_port: old_cfg.wireguard.announced_tunnel_port,
            announced_metadata_port: old_cfg.wireguard.announced_metadata_port,
            private_network_prefix_v4: old_cfg.wireguard.private_network_prefix_v4,
            private_network_prefix_v6: old_cfg.wireguard.private_network_prefix_v6,
            use_userspace: old_cfg.wireguard.use_userspace,
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
        lp: LpConfig {
            control_bind_address: old_cfg.lp.control_bind_address,
            data_bind_address: old_cfg.lp.data_bind_address,
            announce_control_port: old_cfg.lp.announce_control_port,
            announce_data_port: old_cfg.lp.announce_data_port,
            debug: LpDebug {
                max_connections: old_cfg.lp.debug.max_connections,
                use_mock_ecash: old_cfg.lp.debug.use_mock_ecash,
                handshake_ttl: old_cfg.lp.debug.handshake_ttl,
                session_ttl: old_cfg.lp.debug.session_ttl,
                state_cleanup_interval: old_cfg.lp.debug.state_cleanup_interval,
                max_concurrent_forwards: old_cfg.lp.debug.max_concurrent_forwards,
            },
        },
        gateway_tasks: GatewayTasksConfig {
            storage_paths: GatewayTasksPaths {
                clients_storage: old_cfg.gateway_tasks.storage_paths.clients_storage,
                stats_storage: old_cfg.gateway_tasks.storage_paths.stats_storage,
                cosmos_mnemonic: old_cfg.gateway_tasks.storage_paths.cosmos_mnemonic,
                bridge_client_params: old_cfg.gateway_tasks.storage_paths.bridge_client_params,
            },
            enforce_zk_nyms: old_cfg.gateway_tasks.enforce_zk_nyms,
            ws_bind_address: old_cfg.gateway_tasks.ws_bind_address,
            announce_ws_port: old_cfg.gateway_tasks.announce_ws_port,
            announce_wss_port: old_cfg.gateway_tasks.announce_wss_port,
            upgrade_mode: UpgradeModeWatcher {
                enabled: old_cfg.gateway_tasks.upgrade_mode.enabled,
                attestation_url: old_cfg.gateway_tasks.upgrade_mode.attestation_url,
                attester_public_key: old_cfg.gateway_tasks.upgrade_mode.attester_public_key,
                debug: UpgradeModeWatcherDebug {
                    regular_polling_interval: old_cfg
                        .gateway_tasks
                        .upgrade_mode
                        .debug
                        .regular_polling_interval,
                    expedited_poll_interval: old_cfg
                        .gateway_tasks
                        .upgrade_mode
                        .debug
                        .expedited_poll_interval,
                },
            },
            debug: gateway_tasks::Debug {
                message_retrieval_limit: old_cfg.gateway_tasks.debug.message_retrieval_limit,
                maximum_open_connections: old_cfg.gateway_tasks.debug.maximum_open_connections,
                minimum_mix_performance: old_cfg.gateway_tasks.debug.minimum_mix_performance,
                maximum_initial_topology_waiting_time: old_cfg
                    .gateway_tasks
                    .debug
                    .maximum_initial_topology_waiting_time,
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
                upgrade_mode_min_staleness_recheck: old_cfg
                    .gateway_tasks
                    .debug
                    .upgrade_mode_min_staleness_recheck,
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
                allow_local_ips: false,
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
                allow_local_ips: false,
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
        metrics: MetricsConfig {
            debug: metrics::Debug {
                log_stats_to_console: old_cfg.metrics.debug.log_stats_to_console,
                aggregator_update_rate: old_cfg.metrics.debug.aggregator_update_rate,
                stale_mixnet_metrics_cleaner_rate: old_cfg
                    .metrics
                    .debug
                    .stale_mixnet_metrics_cleaner_rate,
                global_prometheus_counters_update_rate: old_cfg
                    .metrics
                    .debug
                    .global_prometheus_counters_update_rate,
                pending_egress_packets_update_rate: old_cfg
                    .metrics
                    .debug
                    .pending_egress_packets_update_rate,
                clients_sessions_update_rate: old_cfg.metrics.debug.clients_sessions_update_rate,
                console_logging_update_interval: old_cfg
                    .metrics
                    .debug
                    .console_logging_update_interval,
                legacy_mixing_metrics_update_rate: old_cfg
                    .metrics
                    .debug
                    .legacy_mixing_metrics_update_rate,
            },
        },
        logging: LoggingSettings {},
        debug: Debug {
            topology_cache_ttl: old_cfg.debug.topology_cache_ttl,
            routing_nodes_check_interval: old_cfg.debug.routing_nodes_check_interval,
            testnet: old_cfg.debug.testnet,
        },
    };
    Ok(cfg)
}
