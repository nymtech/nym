#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

pub mod setup;

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    mut task_client: nym_task::TaskClient,
    gateway_client_registry: std::sync::Arc<
        nym_wireguard_types::registration::GatewayClientRegistry,
    >,
) -> Result<defguard_wireguard_rs::WGApi, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use crate::setup::{peer_allowed_ips, peer_static_public_key, PRIVATE_KEY};
    use defguard_wireguard_rs::{
        host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
    };
    use nym_network_defaults::{WG_PORT, WG_TUN_DEVICE_ADDRESS};

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
    let mut peers = vec![];
    for peer_client in gateway_client_registry.iter() {
        let mut peer = Peer::new(Key::new(peer_client.pub_key.to_bytes()));
        let peer_ip_mask = IpAddrMask::new(peer_client.private_ip, 32);
        peer.set_allowed_ips(vec![peer_ip_mask]);
        wgapi.configure_peer(&peer)?;
        peers.push(peer);
    }
    wgapi.configure_peer_routing(&peers)?;

    tokio::spawn(async move { task_client.recv().await });

    Ok(wgapi)
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard() {
    todo!("WireGuard is currently only supported on Linux");
}
