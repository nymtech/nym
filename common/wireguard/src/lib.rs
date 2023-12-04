#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

pub mod setup;

use nym_wireguard_types::registration::GatewayClientRegistry;
use std::sync::Arc;

// Currently the module related to setting up the virtual network device is platform specific.
#[cfg(target_os = "linux")]
use crate::setup::{peer_allowed_ips, peer_static_public_key, PRIVATE_KEY};
use defguard_wireguard_rs::WGApi;
#[cfg(target_os = "linux")]
use defguard_wireguard_rs::{
    host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WireguardInterfaceApi,
};
#[cfg(target_os = "linux")]
use nym_network_defaults::{WG_PORT, WG_TUN_DEVICE_ADDRESS};

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    mut task_client: nym_task::TaskClient,
    _gateway_client_registry: Arc<GatewayClientRegistry>,
) -> Result<WGApi, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let ifname = String::from("wg0");
    let wgapi = WGApi::new(ifname.clone(), false)?;
    wgapi.create_interface()?;
    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: PRIVATE_KEY.to_string(),
        address: WG_TUN_DEVICE_ADDRESS.to_string(),
        port: WG_PORT as u32,
        peers: vec![],
    };
    wgapi.configure_interface(&interface_config)?;
    let peer = peer_static_public_key();
    let mut peer = Peer::new(Key::new(peer.to_bytes()));
    let peer_ip = peer_allowed_ips();
    let peer_ip_mask = IpAddrMask::new(peer_ip.network_address(), peer_ip.netmask());
    peer.set_allowed_ips(vec![peer_ip_mask]);
    wgapi.configure_peer(&peer)?;
    wgapi.configure_peer_routing(&[peer.clone()])?;

    tokio::spawn(async move { task_client.recv().await });

    Ok(wgapi)
}
#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard(
    _task_client: nym_task::TaskClient,
    _gateway_client_registry: Arc<GatewayClientRegistry>,
) -> Result<WGApi, Box<dyn std::error::Error + Send + Sync + 'static>> {
    todo!("WireGuard is currently only supported on Linux")
}
