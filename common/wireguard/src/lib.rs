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

use nym_tun::tun_task_channel;

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
    // TODO: make this configurable

    // We can optionally index peers by their IP like standard wireguard. If we don't then we do
    // plain NAT where we match incoming destination IP with outgoing source IP.

    use nym_wireguard_types::tun_common::network_table::NetworkTable;
    let peers_by_ip = Arc::new(tokio::sync::Mutex::new(NetworkTable::new()));

    // Alternative 1:
    let routing_mode = tun_device::RoutingMode::new_allowed_ips(peers_by_ip.clone());
    // Alternative 2:
    //let routing_mode = tun_device::RoutingMode::new_nat();

    // Start the tun device that is used to relay traffic outbound
    let config = tun_device::TunDeviceConfig {
        base_name: nym_wireguard_types::WG_TUN_BASE_NAME.to_string(),
        ip: nym_wireguard_types::WG_TUN_DEVICE_ADDRESS.parse().unwrap(),
        netmask: nym_wireguard_types::WG_TUN_DEVICE_NETMASK.parse().unwrap(),
    };
    let (tun, tun_task_tx, tun_task_response_rx) = tun_device::TunDevice::new(routing_mode, config);
    tun.start();

    // We also index peers by a tag
    let peers_by_tag = Arc::new(tokio::sync::Mutex::new(wg_tunnel::PeersByTag::new()));

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
