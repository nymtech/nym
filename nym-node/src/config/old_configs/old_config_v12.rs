// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::helpers::log_error_and_return;
use crate::config::old_configs::old_config_v13::{
    ConfigV13, GatewayTasksConfigDebugV13, GatewayTasksConfigV13, KeysPathsV13, LpConfigV13,
    LpDebugV13, NymNodePathsV13,
};
use crate::error::NymNodeError;
use crate::node::helpers::{
    store_mceliece_keypair, store_mlkem768_keypair, store_x25519_lp_keypair,
};
use nym_config::defaults::{mainnet, var_names};
use nym_config::read_config_from_toml_file;
use nym_config::serde_helpers::de_maybe_port;
use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
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

use crate::config::persistence::{
    DEFAULT_MCELIECE_PRIVATE_KEY_FILENAME, DEFAULT_MCELIECE_PUBLIC_KEY_FILENAME,
    DEFAULT_MLKEM768_PRIVATE_KEY_FILENAME, DEFAULT_MLKEM768_PUBLIC_KEY_FILENAME,
    DEFAULT_X25519_PRIVATE_LP_KEY_FILENAME, DEFAULT_X25519_PUBLIC_LP_KEY_FILENAME,
};
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
) -> Result<ConfigV13, NymNodeError> {
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

    let updated_keys = KeysPathsV13 {
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

    info!("generating new mceliece keypair (this might take a while)");
    let mceliece = generate_keypair_mceliece(&mut rng);
    let paths = updated_keys.mceliece_key_paths();
    store_mceliece_keypair(&mceliece, &paths)?;

    let cfg = ConfigV13 {
        save_path: old_cfg.save_path,
        id: old_cfg.id,
        modes: old_cfg.modes,
        host: old_cfg.host,
        mixnet: old_cfg.mixnet,
        // \/ UPDATED
        storage_paths: NymNodePathsV13 {
            keys: updated_keys,
            description: old_cfg.storage_paths.description,
        },
        // /\ UPDATED
        http: old_cfg.http,
        verloc: old_cfg.verloc,
        wireguard: old_cfg.wireguard,
        lp: LpConfigV13 {
            control_bind_address: old_cfg.gateway_tasks.lp.control_bind_address,
            data_bind_address: old_cfg.gateway_tasks.lp.data_bind_address,
            announce_control_port: old_cfg.gateway_tasks.lp.announce_control_port,
            announce_data_port: old_cfg.gateway_tasks.lp.announce_data_port,
            debug: LpDebugV13 {
                max_connections: old_cfg.gateway_tasks.lp.debug.max_connections,
                use_mock_ecash: old_cfg.gateway_tasks.lp.debug.use_mock_ecash,
                handshake_ttl: old_cfg.gateway_tasks.lp.debug.handshake_ttl,
                session_ttl: old_cfg.gateway_tasks.lp.debug.session_ttl,
                state_cleanup_interval: old_cfg.gateway_tasks.lp.debug.state_cleanup_interval,
                max_concurrent_forwards: old_cfg.gateway_tasks.lp.debug.max_concurrent_forwards,
            },
        },
        gateway_tasks: GatewayTasksConfigV13 {
            storage_paths: old_cfg.gateway_tasks.storage_paths,
            enforce_zk_nyms: old_cfg.gateway_tasks.enforce_zk_nyms,
            ws_bind_address: old_cfg.gateway_tasks.ws_bind_address,
            announce_ws_port: old_cfg.gateway_tasks.announce_ws_port,
            announce_wss_port: old_cfg.gateway_tasks.announce_wss_port,
            upgrade_mode: old_cfg.gateway_tasks.upgrade_mode,
            debug: GatewayTasksConfigDebugV13 {
                message_retrieval_limit: old_cfg.gateway_tasks.debug.message_retrieval_limit,
                maximum_open_connections: old_cfg.gateway_tasks.debug.maximum_open_connections,
                minimum_mix_performance: old_cfg.gateway_tasks.debug.minimum_mix_performance,
                // \/ ADDED
                maximum_initial_topology_waiting_time: GatewayTasksConfigDebugV13::default()
                    .maximum_initial_topology_waiting_time,
                // /\ ADDED
                max_request_timestamp_skew: old_cfg.gateway_tasks.debug.max_request_timestamp_skew,
                stale_messages: old_cfg.gateway_tasks.debug.stale_messages,
                client_bandwidth: old_cfg.gateway_tasks.debug.client_bandwidth,
                zk_nym_tickets: old_cfg.gateway_tasks.debug.zk_nym_tickets,
                upgrade_mode_min_staleness_recheck: old_cfg
                    .gateway_tasks
                    .debug
                    .upgrade_mode_min_staleness_recheck,
            },
        },
        service_providers: old_cfg.service_providers,
        metrics: old_cfg.metrics,
        logging: old_cfg.logging,
        debug: old_cfg.debug,
    };
    Ok(cfg)
}
