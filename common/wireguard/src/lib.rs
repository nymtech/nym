#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    mut task_client: nym_task::TaskClient,
    wireguard_data: std::sync::Arc<nym_wireguard_types::WireguardGatewayData>,
) -> Result<defguard_wireguard_rs::WGApi, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use defguard_wireguard_rs::{
        host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
    };

    let mut peers = vec![];
    for peer_client in wireguard_data.client_registry().iter() {
        let mut peer = Peer::new(Key::new(peer_client.pub_key.to_bytes()));
        let peer_ip_mask = IpAddrMask::new(peer_client.private_ip, 32);
        peer.set_allowed_ips(vec![peer_ip_mask]);
        peers.push(peer);
    }

    let ifname = String::from("wg0");
    let wgapi = WGApi::new(ifname.clone(), false)?;
    wgapi.create_interface()?;
    log::info!("Created interface");
    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: wireguard_data.keypair().private_key().to_string(),
        address: wireguard_data.config().private_ip.to_string(),
        port: wireguard_data.config().announced_port as u32,
        peers,
    };
    log::info!("Configuring with {:?}", interface_config);
    wgapi.configure_interface(&interface_config)?;
    log::info!("Configured interface");
    // wgapi.configure_peer_routing(&peers)?;

    tokio::spawn(async move { task_client.recv().await });

    Ok(wgapi)
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard() {
    todo!("WireGuard is currently only supported on Linux");
}
