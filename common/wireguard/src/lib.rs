#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

mod error;
mod packet_relayer;
mod registered_peers;
pub mod setup;
mod udp_listener;
mod wg_tunnel;

use nym_wireguard_types::registration::GatewayClientRegistry;
use std::sync::Arc;

// Currently the module related to setting up the virtual network device is platform specific.
#[cfg(target_os = "linux")]
use nym_tun::tun_device;

use defguard_wireguard_rs::{host::Peer, InterfaceConfiguration, WGApi, WireguardInterfaceApi};
use nym_network_defaults::{WG_PORT, WG_TUN_DEVICE_ADDRESS};
use nym_tun::tun_task_channel;
use setup::PRIVATE_KEY;

/// Start wireguard device
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
    let peer = std::env::var("NYM_PEER_PUBLIC_KEY").expect("NYM_PEER_PUBLIC_KEY must be set");
    let mut peer = Peer::new(peer.as_str().try_into().unwrap());
    peer.set_allowed_ips(vec!["10.1.0.2".parse().unwrap()]);
    wgapi.configure_peer(&peer)?;
    wgapi.configure_peer_routing(&vec![peer.clone()])?;

    tokio::spawn(async move { task_client.recv().await });

    Ok(wgapi)
}
