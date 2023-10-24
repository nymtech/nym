#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

mod active_peers;
mod error;
mod event;
mod network_table;
mod packet_relayer;
mod platform;
mod registered_peers;
mod setup;
mod tun_task_channel;
mod udp_listener;
mod wg_tunnel;

use nym_wireguard_types::registration::GatewayClientRegistry;
use std::sync::Arc;

// Currently the module related to setting up the virtual network device is platform specific.
#[cfg(target_os = "linux")]
use platform::linux::tun_device;

/// Start wireguard UDP listener and TUN device
///
/// # Errors
///
/// This function will return an error if either the UDP listener of the TUN device fails to start.
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    task_client: nym_task::TaskClient,
    gateway_client_registry: Arc<GatewayClientRegistry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // We can either index peers by their IP like standard wireguard
    let peers_by_ip = Arc::new(tokio::sync::Mutex::new(network_table::NetworkTable::new()));

    // ... or by their tunnel tag, which is a random number assigned to them
    let peers_by_tag = Arc::new(tokio::sync::Mutex::new(wg_tunnel::PeersByTag::new()));

    // Start the tun device that is used to relay traffic outbound
    let (tun, tun_task_tx, tun_task_response_rx) = tun_device::TunDevice::new(peers_by_ip.clone());
    tun.start();

    // If we want to have the tun device on a separate host, it's the tun_task and
    // tun_task_response channels that needs to be sent over the network to the host where the tun
    // device is running.

    // The packet relayer's responsibility is to route packets between the correct tunnel and the
    // tun device. The tun device may or may not be on a separate host, which is why we can't do
    // this routing in the tun device itself.
    let (packet_relayer, packet_tx) = packet_relayer::PacketRelayer::new(
        tun_task_tx.clone(),
        tun_task_response_rx,
        peers_by_tag.clone(),
    );
    packet_relayer.start();

    // Start the UDP listener that clients connect to
    let udp_listener = udp_listener::WgUdpListener::new(
        packet_tx,
        peers_by_ip,
        peers_by_tag,
        Arc::clone(&gateway_client_registry),
    )
    .await?;
    udp_listener.start(task_client);

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard(
    _task_client: nym_task::TaskClient,
    _gateway_client_registry: Arc<GatewayClientRegistry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    todo!("WireGuard is currently only supported on Linux")
}
