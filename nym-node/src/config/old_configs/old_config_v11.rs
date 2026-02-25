// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::old_configs::old_config_v12::{
    ConfigV12, DebugV12, GatewayTasksConfigDebugV12, GatewayTasksConfigV12, LpConfigV12,
    UpgradeModeWatcherV12, WireguardV12,
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
) -> Result<ConfigV12, NymNodeError> {
    debug!("attempting to load v11 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV11::read_from_path(&path)?
    };

    // for future reference: when creating v12 migration,
    // look at how v10 -> v11 is implemented
    // you might be able to create a bunch of type aliases again to save you some headache
    let cfg = ConfigV12 {
        save_path: old_cfg.save_path,
        id: old_cfg.id,
        modes: old_cfg.modes,
        host: old_cfg.host,
        mixnet: old_cfg.mixnet,
        storage_paths: old_cfg.storage_paths,
        http: old_cfg.http,
        verloc: old_cfg.verloc,
        wireguard: WireguardV12 {
            enabled: old_cfg.wireguard.enabled,
            bind_address: old_cfg.wireguard.bind_address,
            private_ipv4: old_cfg.wireguard.private_ipv4,
            private_ipv6: old_cfg.wireguard.private_ipv6,
            announced_tunnel_port: old_cfg.wireguard.announced_tunnel_port,
            announced_metadata_port: old_cfg.wireguard.announced_metadata_port,
            private_network_prefix_v4: old_cfg.wireguard.private_network_prefix_v4,
            private_network_prefix_v6: old_cfg.wireguard.private_network_prefix_v6,
            // \/ ADDED
            use_userspace: false,
            // /\ ADDED
            storage_paths: old_cfg.wireguard.storage_paths,
        },
        gateway_tasks: GatewayTasksConfigV12 {
            storage_paths: old_cfg.gateway_tasks.storage_paths,
            enforce_zk_nyms: old_cfg.gateway_tasks.enforce_zk_nyms,
            ws_bind_address: old_cfg.gateway_tasks.ws_bind_address,
            announce_ws_port: old_cfg.gateway_tasks.announce_ws_port,
            announce_wss_port: old_cfg.gateway_tasks.announce_wss_port,
            // \/ ADDED
            upgrade_mode: UpgradeModeWatcherV12::new()
                .inspect_err(|_| {
                    error!(
                        "failed to set custom upgrade mode configuration - falling back to mainnet"
                    )
                })
                .unwrap_or(UpgradeModeWatcherV12::new_mainnet()),
            lp: LpConfigV12::default(),
            // /\ ADDED
            debug: GatewayTasksConfigDebugV12 {
                message_retrieval_limit: old_cfg.gateway_tasks.debug.message_retrieval_limit,
                maximum_open_connections: old_cfg.gateway_tasks.debug.maximum_open_connections,
                minimum_mix_performance: old_cfg.gateway_tasks.debug.minimum_mix_performance,
                max_request_timestamp_skew: old_cfg.gateway_tasks.debug.max_request_timestamp_skew,
                stale_messages: old_cfg.gateway_tasks.debug.stale_messages,
                client_bandwidth: old_cfg.gateway_tasks.debug.client_bandwidth,
                zk_nym_tickets: old_cfg.gateway_tasks.debug.zk_nym_tickets,

                // \/ ADDED (be explicit about the value rather than using ..Default::default()
                upgrade_mode_min_staleness_recheck: GatewayTasksConfigDebugV12::default()
                    .upgrade_mode_min_staleness_recheck,
                // /\ ADDED
            },
        },
        service_providers: old_cfg.service_providers,
        metrics: old_cfg.metrics,
        logging: old_cfg.logging,
        // \/ FIXED
        debug: DebugV12::default(),
        // /\ FIXED
    };
    Ok(cfg)
}
