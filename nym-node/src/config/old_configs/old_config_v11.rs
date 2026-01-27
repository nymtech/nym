// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::authenticator::{Authenticator, AuthenticatorDebug};
use crate::config::gateway_tasks::{
    ClientBandwidthDebug, StaleMessageDebug, UpgradeModeWatcher, ZkNymTicketHandlerDebug,
};
use crate::config::persistence::{
    AuthenticatorPaths, GatewayTasksPaths, IpPacketRouterPaths, KeysPaths, NetworkRequesterPaths,
    NymNodePaths, ReplayProtectionPaths, ServiceProvidersPaths, WireguardPaths,
};
use crate::config::service_providers::{
    IpPacketRouter, IpPacketRouterDebug, NetworkRequester, NetworkRequesterDebug,
};
use crate::config::{
    Config, GatewayTasksConfig, Host, Http, KeyRotation, KeyRotationDebug, Mixnet, MixnetDebug,
    NodeModes, ReplayProtection, ReplayProtectionDebug, ServiceProvidersConfig, Verloc,
    VerlocDebug, Wireguard, gateway_tasks, service_providers,
};
use crate::error::NymNodeError;
use nym_bin_common::logging::LoggingSettings;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use tracing::{debug, error, instrument};

pub use unchanged_v11_types::*;

// (while some of those are technically unused, they might be needed in future migrations,
// thus allow them to exist)
#[allow(dead_code)]
pub mod unchanged_v11_types {
    use crate::config::old_configs::old_config_v10::{
        AuthenticatorDebugV10, AuthenticatorPathsV10, AuthenticatorV10, ClientBandwidthDebugV10,
        DebugV10, GatewayTasksConfigDebugV10, GatewayTasksConfigV10, GatewayTasksPathsV10, HostV10,
        HttpV10, IpPacketRouterDebugV10, IpPacketRouterPathsV10, IpPacketRouterV10,
        KeyRotationDebugV10, KeyRotationV10, KeysPathsV10, LoggingSettingsV10, MetricsConfigV10,
        MetricsDebugV10, MixnetDebugV10, MixnetV10, NetworkRequesterDebugV10,
        NetworkRequesterPathsV10, NetworkRequesterV10, NodeModeV10, NodeModesV10, NymNodePathsV10,
        ReplayProtectionDebugV10, ReplayProtectionPathsV10, ReplayProtectionV10,
        ServiceProvidersConfigDebugV10, ServiceProvidersConfigV10, ServiceProvidersPathsV10,
        StaleMessageDebugV10, VerlocDebugV10, VerlocV10, WireguardPathsV10,
        ZkNymTicketHandlerDebugV10,
    };

    pub type WireguardPathsV11 = WireguardPathsV10;
    pub type NodeModeV11 = NodeModeV10;
    pub type NodeModesV11 = NodeModesV10;
    pub type HostV11 = HostV10;
    pub type KeyRotationDebugV11 = KeyRotationDebugV10;
    pub type KeyRotationV11 = KeyRotationV10;
    pub type MixnetDebugV11 = MixnetDebugV10;
    pub type MixnetV11 = MixnetV10;
    pub type ReplayProtectionV11 = ReplayProtectionV10;
    pub type ReplayProtectionPathsV11 = ReplayProtectionPathsV10;
    pub type ReplayProtectionDebugV11 = ReplayProtectionDebugV10;
    pub type KeysPathsV11 = KeysPathsV10;
    pub type NymNodePathsV11 = NymNodePathsV10;
    pub type HttpV11 = HttpV10;
    pub type VerlocDebugV11 = VerlocDebugV10;
    pub type VerlocV11 = VerlocV10;
    pub type DebugV11 = DebugV10;
    pub type ZkNymTicketHandlerDebugV11 = ZkNymTicketHandlerDebugV10;
    pub type NetworkRequesterPathsV11 = NetworkRequesterPathsV10;
    pub type IpPacketRouterPathsV11 = IpPacketRouterPathsV10;
    pub type AuthenticatorPathsV11 = AuthenticatorPathsV10;
    pub type AuthenticatorV11 = AuthenticatorV10;
    pub type AuthenticatorDebugV11 = AuthenticatorDebugV10;
    pub type IpPacketRouterDebugV11 = IpPacketRouterDebugV10;
    pub type IpPacketRouterV11 = IpPacketRouterV10;
    pub type NetworkRequesterDebugV11 = NetworkRequesterDebugV10;
    pub type NetworkRequesterV11 = NetworkRequesterV10;
    pub type GatewayTasksPathsV11 = GatewayTasksPathsV10;
    pub type StaleMessageDebugV11 = StaleMessageDebugV10;
    pub type ClientBandwidthDebugV11 = ClientBandwidthDebugV10;
    pub type GatewayTasksConfigDebugV11 = GatewayTasksConfigDebugV10;
    pub type GatewayTasksConfigV11 = GatewayTasksConfigV10;
    pub type ServiceProvidersPathsV11 = ServiceProvidersPathsV10;
    pub type ServiceProvidersConfigDebugV11 = ServiceProvidersConfigDebugV10;
    pub type ServiceProvidersConfigV11 = ServiceProvidersConfigV10;
    pub type MetricsConfigV11 = MetricsConfigV10;
    pub type MetricsDebugV11 = MetricsDebugV10;
    pub type LoggingSettingsV11 = LoggingSettingsV10;
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardV11 {
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

    /// Paths for wireguard keys, client registries, etc.
    pub storage_paths: WireguardPathsV11,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV11 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModesV11,

    pub host: HostV11,

    pub mixnet: MixnetV11,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePathsV11,

    #[serde(default)]
    pub http: HttpV11,

    #[serde(default)]
    pub verloc: VerlocV11,

    pub wireguard: WireguardV11,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfigV11,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfigV11,

    #[serde(default)]
    pub metrics: MetricsConfigV11,

    #[serde(default)]
    pub logging: LoggingSettingsV11,

    #[serde(default)]
    pub debug: DebugV11,
}

impl ConfigV11 {
    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: ConfigV11 =
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
pub async fn try_upgrade_config_v11<P: AsRef<Path>>(
    path: P,
    prev_config: Option<ConfigV11>,
) -> Result<Config, NymNodeError> {
    debug!("attempting to load v11 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV11::read_from_path(&path)?
    };

    // for future reference: when creating v12 migration,
    // look at how v10 -> v11 is implemented
    // you might be able to create a bunch of type aliases again to save you some headache
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
                private_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .private_x25519_noise_key_file,
                public_x25519_noise_key_file: old_cfg
                    .storage_paths
                    .keys
                    .public_x25519_noise_key_file,
                secondary_x25519_sphinx_key_file: old_cfg
                    .storage_paths
                    .keys
                    .secondary_x25519_sphinx_key_file,
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
            use_userspace: false,
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
            // \/ ADDED
            upgrade_mode: UpgradeModeWatcher::new()
                .inspect_err(|_| {
                    error!(
                        "failed to set custom upgrade mode configuration - falling back to mainnet"
                    )
                })
                .unwrap_or(UpgradeModeWatcher::new_mainnet()),
            lp: Default::default(),
            // /\ ADDED
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
                // \/ ADDED (be explicit about the value rather than using ..Default::default()
                upgrade_mode_min_staleness_recheck: gateway_tasks::Debug::default()
                    .upgrade_mode_min_staleness_recheck,
                // /\ ADDED
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
