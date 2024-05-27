#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

use std::sync::Arc;

const WG_TUN_NAME: &str = "nymwg";

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    task_client: nym_task::TaskClient,
    wireguard_data: nym_wireguard_types::WireguardData,
) -> Result<
    Arc<nym_wireguard_types::WgApiWrapper>,
    Box<dyn std::error::Error + Send + Sync + 'static>,
> {
    use base64::{prelude::BASE64_STANDARD, Engine};
    use defguard_wireguard_rs::{
        host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WGApi, WireguardInterfaceApi,
    };
    use nym_wireguard_types::peer_controller::PeerController;

    let mut peers = vec![];
    for peer_client in wireguard_data.inner.client_registry().iter() {
        let mut peer = Peer::new(Key::new(peer_client.pub_key.to_bytes()));
        let peer_ip_mask = IpAddrMask::new(peer_client.private_ip, 32);
        peer.set_allowed_ips(vec![peer_ip_mask]);
        peers.push(peer);
    }

    let ifname = String::from(WG_TUN_NAME);
    let wg_api = WGApi::new(ifname.clone(), false)?;
    wg_api.create_interface()?;
    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: BASE64_STANDARD.encode(wireguard_data.inner.keypair().private_key().to_bytes()),
        address: wireguard_data.inner.config().private_ip.to_string(),
        port: wireguard_data.inner.config().announced_port as u32,
        peers,
    };
    wg_api.configure_interface(&interface_config)?;
    // wgapi.configure_peer_routing(&peers)?;

    let wg_api = Arc::new(nym_wireguard_types::WgApiWrapper::new(wg_api));
    let mut controller = PeerController::new(wg_api.clone(), wireguard_data.peer_rx);
    tokio::spawn(async move { controller.run(task_client).await });

    Ok(wg_api)
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard() {
    todo!("WireGuard is currently only supported on Linux");
}
