// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::authenticator::{Authenticator, AuthenticatorDebug};
use crate::config::gateway_tasks::{
    ClientBandwidthDebug, StaleMessageDebug, UpgradeModeWatcher, UpgradeModeWatcherDebug,
    ZkNymTicketHandlerDebug,
};
use crate::config::helpers::log_error_and_return;
use crate::config::persistence::{
    AuthenticatorPaths, DEFAULT_MCELIECE_PRIVATE_KEY_FILENAME,
    DEFAULT_MCELIECE_PUBLIC_KEY_FILENAME, DEFAULT_MLKEM768_PRIVATE_KEY_FILENAME,
    DEFAULT_MLKEM768_PUBLIC_KEY_FILENAME, DEFAULT_X25519_PRIVATE_LP_KEY_FILENAME,
    DEFAULT_X25519_PUBLIC_LP_KEY_FILENAME, GatewayTasksPaths, IpPacketRouterPaths, KeysPaths,
    NetworkRequesterPaths, NymNodePaths, ReplayProtectionPaths, ServiceProvidersPaths,
    WireguardPaths,
};
use crate::config::service_providers::{
    IpPacketRouter, IpPacketRouterDebug, NetworkRequester, NetworkRequesterDebug,
};
use crate::config::{
    Config, Debug, GatewayTasksConfig, Host, Http, KeyRotation, KeyRotationDebug, MetricsConfig,
    Mixnet, MixnetDebug, NodeModes, ReplayProtection, ReplayProtectionDebug,
    ServiceProvidersConfig, Verloc, VerlocDebug, Wireguard, gateway_tasks, metrics,
    service_providers,
};
use crate::error::NymNodeError;
use crate::node::helpers::{
    store_mceliece_keypair, store_mlkem768_keypair, store_x25519_lp_keypair,
};
use nym_bin_common::logging::LoggingSettings;
use nym_config::defaults::{mainnet, var_names};
use nym_config::read_config_from_toml_file;
use nym_config::serde_helpers::de_maybe_port;
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_gateway::node::LpConfig;
use nym_gateway::node::lp_listener::LpDebug;
use nym_kkt::key_utils::{
    generate_keypair_mceliece, generate_keypair_mlkem, generate_lp_keypair_x25519,
};
use rand09::SeedableRng;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info, instrument};
use url::Url;

pub use unchanged_v12_types::*;

// (while some of those are technically unused, they might be needed in future migrations,
// thus allow them to exist)
#[allow(dead_code)]
pub mod unchanged_v12_types {
    use crate::config::old_configs::old_config_v11::{
        AuthenticatorDebugV11, AuthenticatorPathsV11, AuthenticatorV11, ClientBandwidthDebugV11,
        GatewayTasksPathsV11, HostV11, HttpV11, IpPacketRouterDebugV11, IpPacketRouterPathsV11,
        IpPacketRouterV11, KeyRotationDebugV11, KeyRotationV11, KeysPathsV11, LoggingSettingsV11,
        MetricsConfigV11, MetricsDebugV11, MixnetDebugV11, MixnetV11, NetworkRequesterDebugV11,
        NetworkRequesterPathsV11, NetworkRequesterV11, NodeModeV11, NodeModesV11, NymNodePathsV11,
        ReplayProtectionDebugV11, ReplayProtectionPathsV11, ReplayProtectionV11,
        ServiceProvidersConfigDebugV11, ServiceProvidersConfigV11, ServiceProvidersPathsV11,
        StaleMessageDebugV11, VerlocDebugV11, VerlocV11, WireguardPathsV11,
        ZkNymTicketHandlerDebugV11,
    };

    pub type WireguardPathsV12 = WireguardPathsV11;
    pub type NodeModeV12 = NodeModeV11;
    pub type NodeModesV12 = NodeModesV11;
    pub type HostV12 = HostV11;
    pub type KeyRotationDebugV12 = KeyRotationDebugV11;
    pub type KeyRotationV12 = KeyRotationV11;
    pub type MixnetDebugV12 = MixnetDebugV11;
    pub type MixnetV12 = MixnetV11;
    pub type ReplayProtectionV12 = ReplayProtectionV11;
    pub type ReplayProtectionPathsV12 = ReplayProtectionPathsV11;
    pub type ReplayProtectionDebugV12 = ReplayProtectionDebugV11;
    pub type KeysPathsV12 = KeysPathsV11;
    pub type NymNodePathsV12 = NymNodePathsV11;
    pub type HttpV12 = HttpV11;
    pub type VerlocDebugV12 = VerlocDebugV11;
    pub type VerlocV12 = VerlocV11;
    pub type ZkNymTicketHandlerDebugV12 = ZkNymTicketHandlerDebugV11;
    pub type NetworkRequesterPathsV12 = NetworkRequesterPathsV11;
    pub type IpPacketRouterPathsV12 = IpPacketRouterPathsV11;
    pub type AuthenticatorPathsV12 = AuthenticatorPathsV11;
    pub type AuthenticatorV12 = AuthenticatorV11;
    pub type AuthenticatorDebugV12 = AuthenticatorDebugV11;
    pub type IpPacketRouterDebugV12 = IpPacketRouterDebugV11;
    pub type IpPacketRouterV12 = IpPacketRouterV11;
    pub type NetworkRequesterDebugV12 = NetworkRequesterDebugV11;
    pub type NetworkRequesterV12 = NetworkRequesterV11;
    pub type GatewayTasksPathsV12 = GatewayTasksPathsV11;
    pub type StaleMessageDebugV12 = StaleMessageDebugV11;
    pub type ClientBandwidthDebugV12 = ClientBandwidthDebugV11;
    pub type ServiceProvidersPathsV12 = ServiceProvidersPathsV11;
    pub type ServiceProvidersConfigDebugV12 = ServiceProvidersConfigDebugV11;
    pub type ServiceProvidersConfigV12 = ServiceProvidersConfigV11;
    pub type MetricsConfigV12 = MetricsConfigV11;
    pub type MetricsDebugV12 = MetricsDebugV11;
    pub type LoggingSettingsV12 = LoggingSettingsV11;
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardV12 {
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

    /// Tunnel port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_tunnel_port: u16,

    /// Metadata port announced to external clients wishing to connect to the metadata endpoint.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_metadata_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv4.
    /// The maximum value for IPv4 is 32
    pub private_network_prefix_v4: u8,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv6.
    /// The maximum value for IPv6 is 128
    pub private_network_prefix_v6: u8,

    /// Use userspace implementation of WireGuard (wireguard-go) instead of kernel module.
    /// Useful in containerized environments without kernel WireGuard support.
    /// default: `false`
    #[serde(default)]
    pub use_userspace: bool,

    /// Paths for wireguard keys, client registries, etc.
    pub storage_paths: WireguardPathsV12,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct GatewayTasksConfigDebugV12 {
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

    /// The minimum duration since the last explicit check for the upgrade mode to allow creation of new requests.
    #[serde(with = "humantime_serde")]
    pub upgrade_mode_min_staleness_recheck: Duration,

    pub stale_messages: StaleMessageDebugV12,

    pub client_bandwidth: ClientBandwidthDebugV12,

    pub zk_nym_tickets: ZkNymTicketHandlerDebugV12,
}

impl GatewayTasksConfigDebugV12 {
    pub const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
    pub const DEFAULT_MINIMUM_MIX_PERFORMANCE: u8 = 50;
    pub const DEFAULT_MAXIMUM_AUTH_REQUEST_TIMESTAMP_SKEW: Duration = Duration::from_secs(120);
    pub const DEFAULT_MAXIMUM_OPEN_CONNECTIONS: usize = 8192;
    pub const DEFAULT_UPGRADE_MODE_MIN_STALENESS_RECHECK: Duration = Duration::from_secs(30);
}

impl Default for GatewayTasksConfigDebugV12 {
    fn default() -> Self {
        GatewayTasksConfigDebugV12 {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            maximum_open_connections: Self::DEFAULT_MAXIMUM_OPEN_CONNECTIONS,
            max_request_timestamp_skew: Self::DEFAULT_MAXIMUM_AUTH_REQUEST_TIMESTAMP_SKEW,
            minimum_mix_performance: Self::DEFAULT_MINIMUM_MIX_PERFORMANCE,
            stale_messages: Default::default(),
            client_bandwidth: Default::default(),
            zk_nym_tickets: Default::default(),
            upgrade_mode_min_staleness_recheck: Self::DEFAULT_UPGRADE_MODE_MIN_STALENESS_RECHECK,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UpgradeModeWatcherDebugV12 {
    /// Default polling interval
    #[serde(with = "humantime_serde")]
    pub regular_polling_interval: Duration,

    /// Expedited polling interval for once upgrade mode is detected
    #[serde(with = "humantime_serde")]
    pub expedited_poll_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeModeWatcherV12 {
    /// Specifies whether this gateway watches for upgrade mode changes
    /// via the published attestation file.
    pub enabled: bool,

    /// Endpoint to query to retrieve current upgrade mode attestation.
    pub attestation_url: Url,

    /// Expected public key of the attester providing the upgrade mode attestation
    /// on the specified endpoint
    #[serde(with = "bs58_ed25519_pubkey")]
    pub attester_public_key: ed25519::PublicKey,

    #[serde(default)]
    pub debug: UpgradeModeWatcherDebugV12,
}

impl UpgradeModeWatcherV12 {
    pub fn new_mainnet() -> UpgradeModeWatcherV12 {
        info!("using mainnet configuration for the upgrade mode:");
        info!("\t- url: {}", mainnet::UPGRADE_MODE_ATTESTATION_URL);
        info!(
            "\t- attester public key: {}",
            mainnet::UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY
        );

        // SAFETY:
        // our hardcoded values should always be valid
        #[allow(clippy::expect_used)]
        let attestation_url = mainnet::UPGRADE_MODE_ATTESTATION_URL
            .parse()
            .expect("invalid default upgrade mode attestation URL");

        #[allow(clippy::expect_used)]
        let attester_public_key = mainnet::UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY
            .parse()
            .expect("invalid default upgrade mode attester public key");

        UpgradeModeWatcherV12 {
            enabled: true,
            attestation_url,
            attester_public_key,
            debug: UpgradeModeWatcherDebugV12::default(),
        }
    }

    pub fn new() -> Result<UpgradeModeWatcherV12, NymNodeError> {
        // if env is configured, extract relevant values from there, otherwise fallback to mainnet
        if env::var(var_names::CONFIGURED).is_err() {
            return Ok(Self::new_mainnet());
        }

        // if env is configured, the relevant values should be set
        let Ok(env_attestation_url) = env::var(var_names::UPGRADE_MODE_ATTESTATION_URL) else {
            return log_error_and_return(format!(
                "'{}' is not set whilst the env is set to be configured",
                var_names::UPGRADE_MODE_ATTESTATION_URL
            ));
        };

        let Ok(env_attester_pubkey) =
            env::var(var_names::UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY)
        else {
            return log_error_and_return(format!(
                "'{}' is not set whilst the env is set to be configured",
                var_names::UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY
            ));
        };

        let attestation_url = match env_attestation_url.parse() {
            Ok(url) => url,
            Err(err) => {
                return log_error_and_return(format!(
                    "provided attestation url {env_attestation_url} is invalid: {err}!"
                ));
            }
        };

        let attester_public_key = match env_attester_pubkey.parse() {
            Ok(public_key) => public_key,
            Err(err) => {
                return log_error_and_return(format!(
                    "provided attester public key {env_attester_pubkey} is invalid: {err}!"
                ));
            }
        };

        Ok(UpgradeModeWatcherV12 {
            enabled: true,
            attestation_url,
            attester_public_key,
            debug: UpgradeModeWatcherDebugV12::default(),
        })
    }
}

impl UpgradeModeWatcherDebugV12 {
    const DEFAULT_REGULAR_POLLING_INTERVAL: Duration = Duration::from_secs(15 * 60);
    const DEFAULT_EXPEDITED_POLL_INTERVAL: Duration = Duration::from_secs(2 * 60);
}

impl Default for UpgradeModeWatcherDebugV12 {
    fn default() -> Self {
        UpgradeModeWatcherDebugV12 {
            regular_polling_interval: Self::DEFAULT_REGULAR_POLLING_INTERVAL,
            expedited_poll_interval: Self::DEFAULT_EXPEDITED_POLL_INTERVAL,
        }
    }
}

/// Configuration for LP listener
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpConfigV12 {
    pub control_bind_address: SocketAddr,

    pub data_bind_address: SocketAddr,

    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_control_port: Option<u16>,

    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_data_port: Option<u16>,

    #[serde(default)]
    pub debug: LpDebugV12,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpDebugV12 {
    pub max_connections: usize,
    #[serde(with = "humantime_serde")]
    pub timestamp_tolerance: Duration,
    pub use_mock_ecash: bool,
    #[serde(with = "humantime_serde")]
    pub handshake_ttl: Duration,
    #[serde(with = "humantime_serde")]
    pub session_ttl: Duration,
    #[serde(with = "humantime_serde")]
    pub state_cleanup_interval: Duration,
    pub max_concurrent_forwards: usize,
}

impl LpConfigV12 {
    pub const DEFAULT_CONTROL_PORT: u16 = 41264;
    pub const DEFAULT_DATA_PORT: u16 = 51264;
}

impl Default for LpConfigV12 {
    fn default() -> Self {
        LpConfigV12 {
            control_bind_address: SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                Self::DEFAULT_CONTROL_PORT,
            ),
            data_bind_address: SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                Self::DEFAULT_DATA_PORT,
            ),
            announce_control_port: None,
            announce_data_port: None,
            debug: Default::default(),
        }
    }
}

impl LpDebugV12 {
    pub const DEFAULT_MAX_CONNECTIONS: usize = 10000;
    pub const DEFAULT_TIMESTAMP_TOLERANCE: Duration = Duration::from_secs(30);
    pub const DEFAULT_HANDSHAKE_TTL: Duration = Duration::from_secs(90);
    pub const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(86400);
    pub const DEFAULT_STATE_CLEANUP_INTERVAL: Duration = Duration::from_secs(300);
    pub const DEFAULT_MAX_CONCURRENT_FORWARDS: usize = 1000;
}

impl Default for LpDebugV12 {
    fn default() -> Self {
        LpDebugV12 {
            max_connections: Self::DEFAULT_MAX_CONNECTIONS,
            timestamp_tolerance: Self::DEFAULT_TIMESTAMP_TOLERANCE,
            use_mock_ecash: false,
            handshake_ttl: Self::DEFAULT_HANDSHAKE_TTL,
            session_ttl: Self::DEFAULT_SESSION_TTL,
            state_cleanup_interval: Self::DEFAULT_STATE_CLEANUP_INTERVAL,
            max_concurrent_forwards: Self::DEFAULT_MAX_CONCURRENT_FORWARDS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksConfigV12 {
    pub storage_paths: GatewayTasksPathsV12,

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

    pub upgrade_mode: UpgradeModeWatcherV12,

    #[serde(default)]
    pub lp: LpConfigV12,

    #[serde(default)]
    pub debug: GatewayTasksConfigDebugV12,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugV12 {
    /// Specifies the time to live of the internal topology provider cache.
    #[serde(with = "humantime_serde")]
    pub topology_cache_ttl: Duration,

    /// Specifies the time between attempting to resolve any pending unknown nodes in the routing filter
    #[serde(with = "humantime_serde")]
    pub routing_nodes_check_interval: Duration,

    /// Specifies whether this node runs in testnet mode thus allowing it to route packets on local interfaces
    pub testnet: bool,
}

impl DebugV12 {
    pub const DEFAULT_TOPOLOGY_CACHE_TTL: Duration = Duration::from_secs(10 * 60);
    pub const DEFAULT_ROUTING_NODES_CHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);
}

impl Default for DebugV12 {
    fn default() -> Self {
        DebugV12 {
            topology_cache_ttl: Self::DEFAULT_TOPOLOGY_CACHE_TTL,
            routing_nodes_check_interval: Self::DEFAULT_ROUTING_NODES_CHECK_INTERVAL,
            testnet: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV12 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModesV12,

    pub host: HostV12,

    pub mixnet: MixnetV12,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePathsV12,

    #[serde(default)]
    pub http: HttpV12,

    #[serde(default)]
    pub verloc: VerlocV12,

    pub wireguard: WireguardV12,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfigV12,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfigV12,

    #[serde(default)]
    pub metrics: MetricsConfigV12,

    #[serde(default)]
    pub logging: LoggingSettingsV12,

    #[serde(default)]
    pub debug: DebugV12,
}

impl ConfigV12 {
    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: ConfigV12 =
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
pub async fn try_upgrade_config_v12<P: AsRef<Path>>(
    path: P,
    prev_config: Option<ConfigV12>,
) -> Result<Config, NymNodeError> {
    debug!("attempting to load v12 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV12::read_from_path(&path)?
    };

    info!("migrating the old config (v12)...");
    let keys_dir = old_cfg
        .storage_paths
        .keys
        .public_ed25519_identity_key_file
        .parent()
        .ok_or(NymNodeError::DataDirDerivationFailure)?
        .to_path_buf();

    let updated_keys = KeysPaths {
        private_ed25519_identity_key_file: old_cfg
            .storage_paths
            .keys
            .private_ed25519_identity_key_file,
        public_ed25519_identity_key_file: old_cfg
            .storage_paths
            .keys
            .public_ed25519_identity_key_file,
        primary_x25519_sphinx_key_file: old_cfg.storage_paths.keys.primary_x25519_sphinx_key_file,
        secondary_x25519_sphinx_key_file: old_cfg
            .storage_paths
            .keys
            .secondary_x25519_sphinx_key_file,
        private_x25519_noise_key_file: old_cfg.storage_paths.keys.private_x25519_noise_key_file,
        public_x25519_noise_key_file: old_cfg.storage_paths.keys.public_x25519_noise_key_file,
        private_x25519_lp_key_file: keys_dir.join(DEFAULT_X25519_PRIVATE_LP_KEY_FILENAME),
        public_x25519_lp_key_file: keys_dir.join(DEFAULT_X25519_PUBLIC_LP_KEY_FILENAME),
        private_mlkem768_lp_key_file: keys_dir.join(DEFAULT_MLKEM768_PRIVATE_KEY_FILENAME),
        public_mlkem768_lp_key_file: keys_dir.join(DEFAULT_MLKEM768_PUBLIC_KEY_FILENAME),
        private_mceliece_lp_key_file: keys_dir.join(DEFAULT_MCELIECE_PRIVATE_KEY_FILENAME),
        public_mceliece_lp_key_file: keys_dir.join(DEFAULT_MCELIECE_PUBLIC_KEY_FILENAME),
    };

    let mut rng = rand09::rngs::StdRng::from_os_rng();

    // generate new keys for LP
    info!("generating new LP x25519 DH keypair");
    let x25519 = generate_lp_keypair_x25519(&mut rng);
    let paths = updated_keys.x25519_lp_key_paths();
    store_x25519_lp_keypair(&x25519, &paths)?;

    info!("generating new mlkem768 keypair");
    let mlkem = generate_keypair_mlkem(&mut rng);
    let paths = updated_keys.mlkem768_key_paths();
    store_mlkem768_keypair(&mlkem, &paths)?;

    info!("generating mceliece keypair");
    let mceliece = generate_keypair_mceliece(&mut rng);
    let paths = updated_keys.mceliece_key_paths();
    store_mceliece_keypair(&mceliece, &paths)?;

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
            keys: updated_keys,
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
            lp: LpConfig {
                control_bind_address: old_cfg.gateway_tasks.lp.control_bind_address,
                data_bind_address: old_cfg.gateway_tasks.lp.data_bind_address,
                announce_control_port: old_cfg.gateway_tasks.lp.announce_control_port,
                announce_data_port: old_cfg.gateway_tasks.lp.announce_data_port,
                debug: LpDebug {
                    max_connections: old_cfg.gateway_tasks.lp.debug.max_connections,
                    use_mock_ecash: old_cfg.gateway_tasks.lp.debug.use_mock_ecash,
                    handshake_ttl: old_cfg.gateway_tasks.lp.debug.handshake_ttl,
                    session_ttl: old_cfg.gateway_tasks.lp.debug.session_ttl,
                    state_cleanup_interval: old_cfg.gateway_tasks.lp.debug.state_cleanup_interval,
                    max_concurrent_forwards: old_cfg.gateway_tasks.lp.debug.max_concurrent_forwards,
                },
            },
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
        metrics: MetricsConfig {
            debug: metrics::Debug {
                log_stats_to_console: false,
                aggregator_update_rate: Default::default(),
                stale_mixnet_metrics_cleaner_rate: Default::default(),
                global_prometheus_counters_update_rate: Default::default(),
                pending_egress_packets_update_rate: Default::default(),
                clients_sessions_update_rate: Default::default(),
                console_logging_update_interval: Default::default(),
                legacy_mixing_metrics_update_rate: Default::default(),
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
