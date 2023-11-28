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

use nym_network_defaults::{WG_PORT, WG_TUN_DEVICE_ADDRESS};
use nym_tun::tun_task_channel;
use setup::PRIVATE_KEY;
use wireguard_control::{Backend, Device, DeviceUpdate, Key, KeyPair, PeerConfigBuilder};

#[cfg(target_os = "linux")]
/// Start wireguard device
pub async fn start_wireguard(
    mut task_client: nym_task::TaskClient,
    _gateway_client_registry: Arc<GatewayClientRegistry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let peer = std::env::var("NYM_PEER_PUBLIC_KEY").expect("NYM_PEER_PUBLIC_KEY must be set");
    let peer = PeerConfigBuilder::new(&Key::from_base64(&peer).unwrap())
        .add_allowed_ip("10.1.0.2".parse()?, 32);
    DeviceUpdate::new()
        .set_keypair(KeyPair::from_private(
            Key::from_base64(PRIVATE_KEY).unwrap(),
        ))
        .set_listen_port(WG_PORT)
        .add_peer(peer)
        .apply(&"wg0".parse().unwrap(), Backend::Kernel)
        .unwrap();

    tokio::spawn(async move { task_client.recv().await });

    Ok(())
}
