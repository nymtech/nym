#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

pub mod setup;

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    mut task_client: nym_task::TaskClient,
    _gateway_client_registry: std::sync::Arc<
        nym_wireguard_types::registration::GatewayClientRegistry,
    >,
    peer_pairs: Vec<crate::setup::PeerPair>,
) -> Result<defguard_wireguard_rs::WGApi, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use crate::setup::{peer_static_pairs, PRIVATE_KEY};
    use defguard_wireguard_rs::{InterfaceConfiguration, WGApi, WireguardInterfaceApi};
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
    let peers = peer_static_pairs(peer_pairs);
    for peer in peers.iter() {
        wgapi.configure_peer(peer)?;
    }
    wgapi.configure_peer_routing(&peers)?;

    tokio::spawn(async move { task_client.recv().await });

    Ok(wgapi)
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard() {
    todo!("WireGuard is currently only supported on Linux");
}
