// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use defguard_wireguard_rs::{
    WGApi, WireguardInterfaceApi, error::WireguardInterfaceError, host::Peer, key::Key,
    net::IpAddrMask,
};
use nym_crypto::asymmetric::x25519::KeyPair;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::error;

#[cfg(target_os = "linux")]
use nym_network_defaults::constants::WG_TUN_BASE_NAME;

pub mod error;
mod free_tier_controller;
pub mod ip_pool;
pub mod peer_controller;
pub mod peer_handle;
pub mod peer_storage_manager;

pub use defguard_wireguard_rs::host::Peer as DefguardPeer;
pub use error::Error;
pub use ip_pool::{IpPool, IpPoolError};
pub use nym_wireguard_types::Config as WireguardConfig;
pub use peer_controller::{PeerControlRequest, PeerRegistrationData};

pub const CONTROL_CHANNEL_SIZE: usize = 256;

#[derive(Debug, thiserror::Error)]
pub enum WgApiWrapperError {
    #[error("WireGuard kernel implementation is not available on this platform")]
    KernelUnavailable,

    #[error("WireGuard userspace implementation is not available on this platform")]
    UserspaceUnavailable,

    #[error("WireGuard interface error: {0}")]
    Interface(#[from] WireguardInterfaceError),
}

pub struct WgApiWrapper {
    inner: Box<dyn WireguardInterfaceApi + Sync + Send>,
}

impl WgApiWrapper {
    /// Create new instance of `WgApiWrapper` choosing internal implementation based on `use_userspace` flag and platform availability.
    ///
    /// Falls back to userspace implementation when kernel implementation is requested but not available.
    pub fn new(ifname: &str, use_userspace: bool) -> Result<Self, WgApiWrapperError> {
        if use_userspace {
            Self::userspace(ifname)
        } else {
            Self::kernel(ifname).or_else(|err| {
                if matches!(err, WgApiWrapperError::KernelUnavailable) {
                    Self::userspace(ifname)
                } else {
                    Err(err)
                }
            })
        }
    }

    /// Create userspace implementation
    fn userspace(_ifname: &str) -> Result<Self, WgApiWrapperError> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        {
            let api = WGApi::<defguard_wireguard_rs::Userspace>::new(_ifname)?;
            Ok(Self {
                inner: Box::new(api),
            })
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "netbsd"
        )))]
        {
            Err(WgApiWrapperError::UserspaceUnavailable)
        }
    }

    /// Create kernel implementation if available.
    fn kernel(_ifname: &str) -> Result<Self, WgApiWrapperError> {
        #[cfg(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        {
            let api = WGApi::<defguard_wireguard_rs::Kernel>::new(_ifname)?;
            Ok(Self {
                inner: Box::new(api),
            })
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "freebsd",
            target_os = "netbsd"
        )))]
        {
            Err(WgApiWrapperError::KernelUnavailable)
        }
    }
}

impl Drop for WgApiWrapper {
    fn drop(&mut self) {
        if let Err(e) = self.inner.remove_interface() {
            error!("Could not remove the wireguard interface: {e:?}");
        }
    }
}

impl WireguardInterfaceApi for WgApiWrapper {
    fn create_interface(
        &mut self,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.create_interface()
    }

    fn assign_address(
        &self,
        address: &IpAddrMask,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.assign_address(address)
    }

    fn configure_peer_routing(
        &self,
        peers: &[Peer],
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.configure_peer_routing(peers)
    }

    fn configure_interface(
        &self,
        config: &defguard_wireguard_rs::InterfaceConfiguration,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.configure_interface(config)
    }

    #[cfg(not(windows))]
    fn remove_interface(
        &self,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.remove_interface()
    }

    #[cfg(windows)]
    fn remove_interface(
        &mut self,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.remove_interface()
    }

    fn configure_peer(
        &self,
        peer: &Peer,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.configure_peer(peer)
    }

    fn remove_peer(
        &self,
        peer_pubkey: &Key,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.remove_peer(peer_pubkey)
    }

    fn read_interface_data(
        &self,
    ) -> Result<
        defguard_wireguard_rs::host::Host,
        defguard_wireguard_rs::error::WireguardInterfaceError,
    > {
        self.inner.read_interface_data()
    }

    fn configure_dns(
        &self,
        dns: &[IpAddr],
        search_domains: &[&str],
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.inner.configure_dns(dns, search_domains)
    }
}

#[derive(Clone)]
pub struct WireguardGatewayData {
    config: WireguardConfig,
    keypair: Arc<KeyPair>,
    peer_tx: Sender<PeerControlRequest>,
}

impl WireguardGatewayData {
    pub fn new(
        config: WireguardConfig,
        keypair: Arc<KeyPair>,
    ) -> (Self, Receiver<PeerControlRequest>) {
        let (peer_tx, peer_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        (
            WireguardGatewayData {
                config,
                keypair,
                peer_tx,
            },
            peer_rx,
        )
    }

    pub fn config(&self) -> WireguardConfig {
        self.config
    }

    pub fn keypair(&self) -> &Arc<KeyPair> {
        &self.keypair
    }

    pub fn peer_tx(&self) -> &Sender<PeerControlRequest> {
        &self.peer_tx
    }
}

pub struct WireguardData {
    pub inner: WireguardGatewayData,
    pub peer_rx: Receiver<PeerControlRequest>,
    pub use_userspace: bool,
}

/// Free-tier datapath enforcement parameters, threaded from node config into WireGuard
/// startup so the rate-limit pool + walled garden are scaffolded on the interface.
/// Its presence means the free tier is enabled.
#[derive(Debug, Clone)]
pub struct FreeTierEnforcementConfig {
    /// Aggregate rate-limit pool ceiling, in bytes per second.
    pub pool_bytes_per_second: u64,

    /// Purchase-endpoint allowlist, reachable at full speed from the garden and exempt
    /// from the pool (both address families).
    pub walled_garden_whitelist: Vec<IpAddr>,
}

/// How a persisted free-tier peer should be re-enforced at startup (task 5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReconcileClass {
    /// Active trial: keep it in the rate-limit pool.
    Pool,

    /// Exhausted by time or bytes: confine it to the walled garden.
    Garden,

    /// Not a free peer (no record, or upgraded to paid): no free-tier enforcement.
    Unenforced,
}

/// Classify a persisted peer for the startup reconcile from its free-tier state. A peer
/// is gardened once EITHER limit is spent (mirrors the whichever-first exhaustion in the
/// peer handle), so a returning exhausted peer is confined before it forwards a packet.
fn classify_for_reconcile(
    is_free: bool,
    elapsed_secs: i64,
    available_bandwidth: i64,
    time_cap_secs: i64,
) -> ReconcileClass {
    if !is_free {
        return ReconcileClass::Unenforced;
    }
    if elapsed_secs >= time_cap_secs || available_bandwidth <= 0 {
        ReconcileClass::Garden
    } else {
        ReconcileClass::Pool
    }
}

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    ecash_manager: Arc<dyn nym_credential_verification::ecash::traits::EcashManager + Send + Sync>,
    metrics: nym_node_metrics::NymNodeMetrics,
    peers: Vec<Peer>,
    upgrade_mode_status: nym_credential_verification::upgrade_mode::UpgradeModeStatus,
    shutdown_token: nym_task::ShutdownToken,
    wireguard_data: WireguardData,
    use_userspace: bool,
    free_tier_enforcement: Option<FreeTierEnforcementConfig>,
) -> Result<std::sync::Arc<WgApiWrapper>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use crate::free_tier_controller::FreeTierController;
    use crate::peer_handle::PeerFreeTier;
    use base64::{Engine, prelude::BASE64_STANDARD};
    use defguard_wireguard_rs::InterfaceConfiguration;
    use ip_network::IpNetwork;
    use nym_free_tier_enforcement::PeerAddrs;
    use peer_controller::PeerController;
    use std::collections::HashMap;
    use tokio::sync::RwLock;
    use tracing::info;

    let ifname = String::from(WG_TUN_BASE_NAME);
    info!(
        "Initializing WireGuard interface '{}' with use_userspace={}",
        ifname, use_userspace
    );
    let mut wg_api = WgApiWrapper::new(&ifname, use_userspace)?;
    let mut peer_bandwidth_managers = HashMap::with_capacity(peers.len());

    // Build the free-tier enforcement facade once (when enabled): used to reconcile the
    // datapath below (task 5.5) and handed to the PeerController for per-peer admit /
    // walled-garden transitions (tasks 4.4 / 5.4).
    let free_tier_controller = match free_tier_enforcement {
        Some(cfg) => FreeTierController::new_enabled(ifname.clone(), cfg),
        None => FreeTierController::new_disabled(),
    };

    // Free-tier startup reconcile (task 5.5): while loading persisted peers, partition
    // the free ones into the rate-limit pool (active trial) vs the walled garden
    // (exhausted) so the datapath scaffolding can be rebuilt from state below, before
    // any peer traffic is forwarded.
    let free_tier_enabled = free_tier_controller.free_tier_enabled();
    let now = time::OffsetDateTime::now_utc();
    let time_cap_secs = nym_network_defaults::constants::FREE_TIER_TRIAL_TIME_CAP.as_secs() as i64;
    let mut pooled_peers: Vec<PeerAddrs> = Vec::new();
    let mut gardened_peers: Vec<PeerAddrs> = Vec::new();

    for peer in peers.iter() {
        let bandwidth_manager =
            PeerController::generate_bandwidth_manager(ecash_manager.storage(), &peer.public_key)
                .await?;

        let free_tier_record = ecash_manager
            .storage()
            .get_free_tier_record(&peer.public_key.to_string())
            .await?;
        let peer_free_tier = PeerFreeTier::from_storage_record(free_tier_record);

        if free_tier_enabled
            && let Some(granted_at) = peer_free_tier.granted_at()
            && let Some(ip_pair) = crate::ip_pool::allocated_ip_pair(peer)
        {
            let addrs = ip_pair.as_free_tier_peers();
            let elapsed_secs = (now - granted_at).whole_seconds();
            match classify_for_reconcile(
                peer_free_tier.is_free_tier(),
                elapsed_secs,
                bandwidth_manager.available_bandwidth().await,
                time_cap_secs,
            ) {
                ReconcileClass::Pool => pooled_peers.push(addrs),
                ReconcileClass::Garden => gardened_peers.push(addrs),
                ReconcileClass::Unenforced => {}
            }
        }

        let bandwidth_manager = peer_handle::SharedBandwidthStorageManager::new(
            Arc::new(RwLock::new(bandwidth_manager)),
            peer.allowed_ips.clone(),
        );
        peer_bandwidth_managers.insert(
            peer.public_key.clone(),
            peer_controller::PeerHandleSeed {
                bandwidth_manager,
                peer: peer.clone(),
                free_tier: peer_free_tier,
            },
        );
    }

    // Initialize IP pool from configuration
    info!("Initializing IP pool for WireGuard peer allocation");
    let mut ip_pool = IpPool::new(
        wireguard_data.inner.config().private_ipv4,
        wireguard_data.inner.config().private_network_prefix_v4,
        wireguard_data.inner.config().private_ipv6,
        wireguard_data.inner.config().private_network_prefix_v6,
    )?;

    // Mark existing peer IPs as used in the pool
    for peer in &peers {
        if let Some(ip_pair) = crate::ip_pool::allocated_ip_pair(peer) {
            ip_pool.mark_used(ip_pair)?;
        }
    }

    wg_api.create_interface()?;

    // Bring the interface administratively up before assigning addresses/routes.
    // The kernel backend sets IFF_UP inside create_interface, but the userspace
    // (BoringTun) backend creates the TUN device DOWN and leaves link-up to the
    // caller - without this, the peer routing route add fails with ENETDOWN
    // ("Network is down"). No-op on the kernel path (interface already up).
    std::process::Command::new("ip")
        .args(["link", "set", "dev", &ifname, "up"])
        .output()
        .inspect_err(|e| tracing::error!("Failed to bring up wireguard interface: {e:?}"))?;

    // Free-tier datapath enforcement (task 5.5): build the rate-limit pool + walled
    // garden scaffolding and rebuild per-peer membership from persisted state now - after
    // the interface exists (tc needs it) but before it is configured with peers and
    // begins forwarding - so a returning garden peer is confined from its first packet.
    // `reconcile` teardown-then-creates, so it is idempotent across restarts. A failure
    // here (e.g. `nft`/`tc` missing) IS the startup preflight: surface it and fail node
    // startup, never degrade to serving free peers unrestricted.
    free_tier_controller.reconcile(&pooled_peers, &gardened_peers)?;

    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: BASE64_STANDARD.encode(wireguard_data.inner.keypair().private_key().to_bytes()),
        addresses: vec![IpAddrMask::host(IpAddr::from(
            wireguard_data.inner.config().private_ipv4,
        ))],
        port: wireguard_data.inner.config().announced_tunnel_port,
        peers,
        mtu: None,
    };
    info!(
        "attempting to configure wireguard interface '{ifname}': addresses=[{}], port={}",
        interface_config
            .addresses
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        interface_config.port
    );

    info!("Configuring WireGuard interface...");
    wg_api
        .configure_interface(&interface_config)
        .inspect_err(|e| tracing::error!("Failed to configure WireGuard interface: {:?}", e))?;

    info!("Adding IPv6 address to interface...");
    std::process::Command::new("ip")
        .args([
            "-6",
            "addr",
            "add",
            &format!(
                "{}/{}",
                wireguard_data.inner.config().private_ipv6,
                wireguard_data.inner.config().private_network_prefix_v6
            ),
            "dev",
            (&ifname),
        ])
        .output()
        .inspect_err(|e| tracing::error!("Failed to add IPv6 address: {:?}", e))?;

    // Use a dummy peer to create routing rule for the entire network space
    let mut catch_all_peer = Peer::new(Key::new([0; 32]));
    let network_v4 = IpNetwork::new_truncate(
        wireguard_data.inner.config().private_ipv4,
        wireguard_data.inner.config().private_network_prefix_v4,
    )?;
    let network_v6 = IpNetwork::new_truncate(
        wireguard_data.inner.config().private_ipv6,
        wireguard_data.inner.config().private_network_prefix_v6,
    )?;
    catch_all_peer.set_allowed_ips(vec![
        IpAddrMask::new(network_v4.network_address(), network_v4.netmask()),
        IpAddrMask::new(network_v6.network_address(), network_v6.netmask()),
    ]);
    wg_api.configure_peer_routing(&[catch_all_peer])?;

    let host = wg_api.read_interface_data()?;

    let wg_api = std::sync::Arc::new(wg_api);
    let mut controller = PeerController::new(
        ecash_manager,
        metrics,
        ip_pool,
        wg_api.clone(),
        host,
        peer_bandwidth_managers,
        wireguard_data.inner.peer_tx.clone(),
        wireguard_data.peer_rx,
        upgrade_mode_status,
        free_tier_controller,
        shutdown_token,
    );
    tokio::spawn(async move { controller.run().await });

    Ok(wg_api)
}

#[cfg(test)]
mod reconcile_tests {
    use super::{ReconcileClass, classify_for_reconcile};

    const CAP: i64 = 600; // FREE_TIER_TRIAL_TIME_CAP placeholder for the test

    #[test]
    fn paid_peer_is_unenforced() {
        // is_free == false -> upgraded to paid, no free-tier enforcement even if fresh.
        assert_eq!(
            classify_for_reconcile(false, 0, 100, CAP),
            ReconcileClass::Unenforced
        );
    }

    #[test]
    fn fresh_free_peer_with_bytes_is_pooled() {
        assert_eq!(
            classify_for_reconcile(true, 0, 100, CAP),
            ReconcileClass::Pool
        );
    }

    #[test]
    fn time_exhausted_free_peer_is_gardened() {
        // at the cap and beyond -> gardened (boundary is inclusive, mirrors the handle).
        assert_eq!(
            classify_for_reconcile(true, CAP, 100, CAP),
            ReconcileClass::Garden
        );
        assert_eq!(
            classify_for_reconcile(true, CAP + 1, 100, CAP),
            ReconcileClass::Garden
        );
    }

    #[test]
    fn byte_exhausted_free_peer_is_gardened() {
        // within the time cap but out of bytes -> gardened (whichever-first exhaustion).
        assert_eq!(
            classify_for_reconcile(true, 10, 0, CAP),
            ReconcileClass::Garden
        );
        assert_eq!(
            classify_for_reconcile(true, 10, -5, CAP),
            ReconcileClass::Garden
        );
    }
}
