// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::authenticator::{Authenticator, AuthenticatorDebug};
use crate::config::gateway_tasks::{
    ClientBandwidthDebug, StaleMessageDebug, UpgradeModeWatcher, UpgradeModeWatcherDebug,
    ZkNymTicketHandlerDebug,
};
use crate::config::persistence::{
    AuthenticatorPaths, GatewayTasksPaths, IpPacketRouterPaths, KeysPaths, NetworkRequesterPaths,
    NymNodePaths, ReplayProtectionPaths, ServiceProvidersPaths, WireguardPaths,
};
use crate::config::service_providers::{
    IpPacketRouter, IpPacketRouterDebug, NetworkRequester, NetworkRequesterDebug,
};
use crate::config::{
    Config, Debug, DirectoryConfig, GatewayTasksConfig, Host, Http, KeyRotation, KeyRotationDebug,
    LpConfig, LpDebug, MetricsConfig, Mixnet, MixnetDebug, NodeModes, Nyx, ReplayProtection,
    ReplayProtectionDebug, ServiceProvidersConfig, Verloc, VerlocDebug, Wireguard, gateway_tasks,
    metrics, service_providers,
};
use crate::error::NymNodeError;
use nym_bin_common::logging::LoggingSettings;
use nym_config::defaults::{mainnet, var_names};
use nym_config::serde_helpers::de_maybe_port;
use nym_config::{parse_urls, read_config_from_toml_file};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info, instrument};
use url::Url;

#[allow(unused_imports)]
pub use unchanged_v14_types::*;

// (while some of those are technically unused, they might be needed in future migrations,
// thus allow them to exist)
#[allow(dead_code)]
pub mod unchanged_v14_types {
    use crate::config::old_configs::old_config_v13::unchanged_v13_types::{
        AuthenticatorDebugV13, AuthenticatorPathsV13, AuthenticatorV13, ClientBandwidthDebugV13,
        DebugV13, HostV13, HttpV13, IpPacketRouterDebugV13, IpPacketRouterPathsV13,
        IpPacketRouterV13, KeyRotationDebugV13, KeyRotationV13, LoggingSettingsV13,
        MetricsConfigV13, MetricsDebugV13, NetworkRequesterDebugV13, NetworkRequesterPathsV13,
        NetworkRequesterV13, NodeModeV13, NodeModesV13, ReplayProtectionDebugV13,
        ReplayProtectionPathsV13, ReplayProtectionV13, ServiceProvidersConfigDebugV13,
        ServiceProvidersConfigV13, ServiceProvidersPathsV13, StaleMessageDebugV13,
        UpgradeModeWatcherV13, VerlocDebugV13, VerlocV13, WireguardPathsV13, WireguardV13,
        ZkNymTicketHandlerDebugV13,
    };
    use crate::config::old_configs::old_config_v13::{GatewayTasksConfigDebugV13, LpConfigV13};

    pub type WireguardPathsV14 = WireguardPathsV13;
    pub type NodeModeV14 = NodeModeV13;
    pub type NodeModesV14 = NodeModesV13;
    pub type HostV14 = HostV13;
    pub type KeyRotationDebugV14 = KeyRotationDebugV13;
    pub type KeyRotationV14 = KeyRotationV13;
    pub type ReplayProtectionV14 = ReplayProtectionV13;
    pub type ReplayProtectionPathsV14 = ReplayProtectionPathsV13;
    pub type ReplayProtectionDebugV14 = ReplayProtectionDebugV13;
    pub type HttpV14 = HttpV13;
    pub type VerlocDebugV14 = VerlocDebugV13;
    pub type VerlocV14 = VerlocV13;
    pub type ZkNymTicketHandlerDebugV14 = ZkNymTicketHandlerDebugV13;
    pub type NetworkRequesterPathsV14 = NetworkRequesterPathsV13;
    pub type IpPacketRouterPathsV14 = IpPacketRouterPathsV13;
    pub type AuthenticatorPathsV14 = AuthenticatorPathsV13;
    pub type AuthenticatorV14 = AuthenticatorV13;
    pub type AuthenticatorDebugV14 = AuthenticatorDebugV13;
    pub type IpPacketRouterDebugV14 = IpPacketRouterDebugV13;
    pub type IpPacketRouterV14 = IpPacketRouterV13;
    pub type NetworkRequesterDebugV14 = NetworkRequesterDebugV13;
    pub type NetworkRequesterV14 = NetworkRequesterV13;
    pub type StaleMessageDebugV14 = StaleMessageDebugV13;
    pub type ClientBandwidthDebugV14 = ClientBandwidthDebugV13;
    pub type ServiceProvidersPathsV14 = ServiceProvidersPathsV13;
    pub type ServiceProvidersConfigDebugV14 = ServiceProvidersConfigDebugV13;
    pub type ServiceProvidersConfigV14 = ServiceProvidersConfigV13;
    pub type MetricsConfigV14 = MetricsConfigV13;
    pub type MetricsDebugV14 = MetricsDebugV13;
    pub type LoggingSettingsV14 = LoggingSettingsV13;
    pub type WireguardV14 = WireguardV13;
    pub type DebugV14 = DebugV13;
    pub type UpgradeModeWatcherV14 = UpgradeModeWatcherV13;
    pub type LpConfigV14 = LpConfigV13;
    pub type GatewayTasksConfigDebugV14 = GatewayTasksConfigDebugV13;
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksPathsV14 {
    pub clients_storage: PathBuf,

    pub stats_storage: PathBuf,

    pub bridge_client_params: Option<PathBuf>,

    pub cosmos_mnemonic: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTasksConfigV14 {
    pub storage_paths: GatewayTasksPathsV14,
    pub enforce_zk_nyms: bool,
    pub ws_bind_address: SocketAddr,
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_ws_port: Option<u16>,
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_wss_port: Option<u16>,

    pub upgrade_mode: UpgradeModeWatcherV14,

    #[serde(default)]
    pub debug: GatewayTasksConfigDebugV14,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct KeysPathsV14 {
    pub private_ed25519_identity_key_file: PathBuf,
    pub public_ed25519_identity_key_file: PathBuf,
    pub primary_x25519_sphinx_key_file: PathBuf,
    pub secondary_x25519_sphinx_key_file: PathBuf,
    pub private_x25519_noise_key_file: PathBuf,
    pub public_x25519_noise_key_file: PathBuf,

    // >> LP KEYS START:
    pub private_x25519_lp_key_file: PathBuf,
    pub public_x25519_lp_key_file: PathBuf,
    pub private_mlkem768_lp_key_file: PathBuf,
    pub public_mlkem768_lp_key_file: PathBuf,
    pub private_mceliece_lp_key_file: PathBuf,
    pub public_mceliece_lp_key_file: PathBuf,
    // >> LP KEYS END
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NymNodePathsV14 {
    pub keys: KeysPathsV14,
    pub description: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct MixnetDebugV14 {
    #[serde(with = "humantime_serde")]
    pub maximum_forward_packet_delay: Duration,
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_initial_backoff: Duration,
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_maximum_backoff: Duration,
    #[serde(with = "humantime_serde")]
    pub initial_connection_timeout: Duration,
    #[serde(with = "humantime_serde")]
    pub connection_idle_timeout: Duration,
    #[serde(with = "humantime_serde")]
    pub connection_write_timeout: Duration,
    pub maximum_connection_buffer_size: usize,
    pub use_legacy_packet_encoding: bool,
    pub egress_trace_sample_rate: u64,
    pub unsafe_disable_noise: bool,
}

impl MixnetDebugV14 {
    const DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY: Duration = Duration::from_secs(10);
    const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
    const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_secs(16);
    const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
    const DEFAULT_CONNECTION_IDLE_TIMEOUT: Duration = Duration::from_secs(300);
    const DEFAULT_CONNECTION_WRITE_TIMEOUT: Duration = Duration::from_millis(500);
    const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 192;
    const DEFAULT_EGRESS_TRACE_SAMPLE_RATE: u64 = 100;
}

impl Default for MixnetDebugV14 {
    fn default() -> Self {
        MixnetDebugV14 {
            maximum_forward_packet_delay: Self::DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY,
            packet_forwarding_initial_backoff: Self::DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: Self::DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: Self::DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            connection_idle_timeout: Self::DEFAULT_CONNECTION_IDLE_TIMEOUT,
            connection_write_timeout: Self::DEFAULT_CONNECTION_WRITE_TIMEOUT,
            maximum_connection_buffer_size: Self::DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            egress_trace_sample_rate: Self::DEFAULT_EGRESS_TRACE_SAMPLE_RATE,
            use_legacy_packet_encoding: true,
            unsafe_disable_noise: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixnetV14 {
    pub bind_address: SocketAddr,

    #[serde(deserialize_with = "de_maybe_port")]
    #[serde(default)]
    pub announce_port: Option<u16>,

    pub nym_api_urls: Vec<Url>,

    pub replay_protection: ReplayProtectionV14,

    #[serde(default)]
    pub key_rotation: KeyRotationV14,

    #[serde(default)]
    pub debug: MixnetDebugV14,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct NyxV14 {
    pub nyxd_websocket_url: Url,

    pub nyxd_urls: Vec<Url>,
}

impl Default for NyxV14 {
    fn default() -> Self {
        // SAFETY:
        // our hardcoded values should always be valid
        // is if there's anything set in the environment, otherwise fallback to mainnet
        #[allow(clippy::expect_used)]
        let nyxd_urls = if let Ok(env_value) = env::var(var_names::NYXD_QUERY_LITE) {
            // 1. try the lite node if available
            parse_urls(&env_value)
        } else if let Ok(env_value) = env::var(var_names::NYXD) {
            // 2. then fallback to the main rpc node
            parse_urls(&env_value)
        } else {
            // finally fallback to mainnet nodes
            vec![
                mainnet::NYXD_QUERY_LITE
                    .parse()
                    .expect("invalid default nyxd lite URL"),
                mainnet::NYXD_URL.parse().expect("invalid default nyxd URL"),
            ]
        };

        #[allow(clippy::expect_used)]
        let nyxd_websocket_url = if let Ok(env_value) = env::var(var_names::NYXD_WS_LITE) {
            env_value
                .parse()
                .expect("malformed default nyxd lite websocket URL")
        } else if let Ok(env_value) = env::var(var_names::NYXD_WEBSOCKET) {
            env_value
                .parse()
                .expect("malformed default nyxd websocket URL")
        } else {
            mainnet::NYXD_WS_LITE
                .parse()
                .expect("invalid default mainnet nyxd websocket URL")
        };
        NyxV14 {
            nyxd_websocket_url,
            nyxd_urls,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV14 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModesV14,

    pub host: HostV14,

    pub mixnet: MixnetV14,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePathsV14,

    #[serde(default)]
    pub nyx: NyxV14,

    #[serde(default)]
    pub http: HttpV14,

    #[serde(default)]
    pub verloc: VerlocV14,

    pub wireguard: WireguardV14,

    #[serde(default)]
    pub lp: LpConfigV14,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfigV14,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfigV14,

    #[serde(default)]
    pub metrics: MetricsConfigV14,

    #[serde(default)]
    pub logging: LoggingSettingsV14,

    #[serde(default)]
    pub debug: DebugV14,
}

impl ConfigV14 {
    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: ConfigV14 =
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
pub async fn try_upgrade_config_v14<P: AsRef<Path>>(
    path: P,
    prev_config: Option<ConfigV14>,
) -> Result<Config, NymNodeError> {
    debug!("attempting to load v14 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV14::read_from_path(&path)?
    };

    info!("migrating the old config (v14)...");

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
                connection_idle_timeout: old_cfg.mixnet.debug.connection_idle_timeout,
                connection_write_timeout: old_cfg.mixnet.debug.connection_write_timeout,
                maximum_connection_buffer_size: old_cfg.mixnet.debug.maximum_connection_buffer_size,
                egress_trace_sample_rate: old_cfg.mixnet.debug.egress_trace_sample_rate,
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
            // \/ MOVED
            cosmos_mnemonic: old_cfg.gateway_tasks.storage_paths.cosmos_mnemonic,
            // /\ MOVED
        },
        nyx: Nyx {
            nyxd_websocket_url: old_cfg.nyx.nyxd_websocket_url,
            nyxd_urls: old_cfg.nyx.nyxd_urls,
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
        // \/ ADDED
        directory: DirectoryConfig::default(),
        // /\ ADDED
        logging: LoggingSettings {},
        debug: Debug {
            topology_cache_ttl: old_cfg.debug.topology_cache_ttl,
            routing_nodes_check_interval: old_cfg.debug.routing_nodes_check_interval,
            testnet: old_cfg.debug.testnet,
        },
    };
    Ok(cfg)
}
