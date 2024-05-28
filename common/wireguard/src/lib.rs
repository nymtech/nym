#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

use defguard_wireguard_rs::WGApi;

const WG_TUN_NAME: &str = "nymwg";

pub struct WgApiWrapper {
    wg_api: WGApi,
}

impl WgApiWrapper {
    pub fn new(wg_api: WGApi) -> Self {
        WgApiWrapper { wg_api }
    }
}

impl Drop for WgApiWrapper {
    fn drop(&mut self) {
        if let Err(e) = defguard_wireguard_rs::WireguardInterfaceApi::remove_interface(&self.wg_api)
        {
            log::error!("Could not remove the wireguard interface: {:?}", e);
        }
    }
}

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    mut task_client: nym_task::TaskClient,
    wireguard_data: std::sync::Arc<nym_wireguard_types::WireguardGatewayData>,
) -> Result<WgApiWrapper, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use base64::{prelude::BASE64_STANDARD, Engine};
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

    let ifname = String::from(WG_TUN_NAME);
    let wgapi = WGApi::new(ifname.clone(), false)?;
    wgapi.create_interface()?;
    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: BASE64_STANDARD.encode(wireguard_data.keypair().private_key().to_bytes()),
        address: wireguard_data.config().private_ip.to_string(),
        port: wireguard_data.config().announced_port as u32,
        peers,
    };
    wgapi.configure_interface(&interface_config)?;
    // wgapi.configure_peer_routing(&peers)?;

    tokio::spawn(async move { task_client.recv().await });

    Ok(WgApiWrapper::new(wgapi))
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard() {
    todo!("WireGuard is currently only supported on Linux");
}
