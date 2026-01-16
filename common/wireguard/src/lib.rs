// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

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
use nym_ip_packet_requests::IpPair;

#[cfg(target_os = "linux")]
use nym_network_defaults::constants::WG_TUN_BASE_NAME;

pub mod error;
pub mod ip_pool;
pub mod peer_controller;
pub mod peer_handle;
pub mod peer_storage_manager;

pub use error::Error;
pub use ip_pool::{IpPool, IpPoolError};
pub use nym_wireguard_types::Config as WireguardConfig;
pub use peer_controller::{PeerControlRequest, PeerRegistrationData};

pub const CONTROL_CHANNEL_SIZE: usize = 256;

pub struct WgApiWrapper {
    inner: Box<dyn WireguardInterfaceApi + Sync + Send>,
}

impl WgApiWrapper {
    /// Create new instance of `WgApiWrapper` choosing internal implementation based on `use_userspace` flag and platform availability.
    ///
    /// Falls back to userspace implementation when kernel implementation is requested but not available.
    pub fn new(ifname: &str, use_userspace: bool) -> Result<Self, WireguardInterfaceError> {
        if use_userspace {
            Self::userspace(ifname)
        } else {
            Self::kernel(ifname)
                .transpose()
                .unwrap_or_else(|| Self::userspace(ifname))
        }
    }

    /// Create userspace implementation
    fn userspace(ifname: &str) -> Result<Self, WireguardInterfaceError> {
        let api = WGApi::<defguard_wireguard_rs::Userspace>::new(ifname)?;
        Ok(Self {
            inner: Box::new(api),
        })
    }

    /// Create kernel implementation if available.
    ///
    /// Returns `None` if kernel implementation is not available.
    ///
    /// See platforms where kernel implementation is available:
    /// <https://github.com/DefGuard/wireguard-rs>
    fn kernel(_ifname: &str) -> Result<Option<Self>, WireguardInterfaceError> {
        #[cfg(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        {
            let api = WGApi::<defguard_wireguard_rs::Kernel>::new(_ifname)?;
            Ok(Some(Self {
                inner: Box::new(api),
            }))
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "freebsd",
            target_os = "netbsd"
        )))]
        {
            Ok(None)
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
) -> Result<std::sync::Arc<WgApiWrapper>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use base64::{Engine, prelude::BASE64_STANDARD};
    use defguard_wireguard_rs::{InterfaceConfiguration, WireguardInterfaceApi};
    use ip_network::IpNetwork;
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

    for peer in peers.iter() {
        let bandwidth_manager = peer_handle::SharedBandwidthStorageManager::new(
            Arc::new(RwLock::new(
                PeerController::generate_bandwidth_manager(
                    ecash_manager.storage(),
                    &peer.public_key,
                )
                .await?,
            )),
            peer.allowed_ips.clone(),
        );
        peer_bandwidth_managers.insert(peer.public_key.clone(), (bandwidth_manager, peer.clone()));
    }

    wg_api.create_interface()?;
    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: BASE64_STANDARD.encode(wireguard_data.inner.keypair().private_key().to_bytes()),
        addresses: vec![IpAddrMask::host(IpAddr::from(
            wireguard_data.inner.config().private_ipv4,
        ))],
        port: wireguard_data.inner.config().announced_tunnel_port,
        peers: peers.clone(), // Clone since we need to use peers later to mark IPs as used
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

    // Initialize IP pool from configuration
    info!("Initializing IP pool for WireGuard peer allocation");
    let ip_pool = IpPool::new(
        wireguard_data.inner.config().private_ipv4,
        wireguard_data.inner.config().private_network_prefix_v4,
        wireguard_data.inner.config().private_ipv6,
        wireguard_data.inner.config().private_network_prefix_v6,
    )?;

    // Mark existing peer IPs as used in the pool
    for peer in &peers {
        for allowed_ip in &peer.allowed_ips {
            // Extract IPv4 and IPv6 from peer's allowed_ips
            if let IpAddr::V4(ipv4) = allowed_ip.address {
                // Find corresponding IPv6
                if let Some(ipv6_mask) = peer
                    .allowed_ips
                    .iter()
                    .find(|ip| matches!(ip.address, IpAddr::V6(_)))
                    && let IpAddr::V6(ipv6) = ipv6_mask.address
                {
                    ip_pool.mark_used(IpPair::new(ipv4, ipv6)).await;
                }
            }
        }
    }

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
        shutdown_token,
    );
    tokio::spawn(async move { controller.run().await });

    Ok(wg_api)
}
