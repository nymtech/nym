#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

mod active_peers;
mod error;
mod event;
mod network_table;
mod platform;
mod registered_peers;
mod setup;
mod udp_listener;
mod wg_tunnel;

use nym_types::gateway_client_registration::GatewayClientRegistry;
use std::sync::Arc;

// Currently the module related to setting up the virtual network device is platform specific.
#[cfg(target_os = "linux")]
use platform::linux::tun_device;

#[derive(Clone)]
pub struct TunTaskTx(tokio::sync::mpsc::UnboundedSender<Vec<u8>>);

impl TunTaskTx {
    fn send(&self, packet: Vec<u8>) -> Result<(), tokio::sync::mpsc::error::SendError<Vec<u8>>> {
        self.0.send(packet)
    }
}

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
    use std::sync::Arc;

    let peers_by_ip = Arc::new(std::sync::Mutex::new(network_table::NetworkTable::new()));

    // Start the tun device that is used to relay traffic outbound
    let (tun, tun_task_tx) = tun_device::TunDevice::new(peers_by_ip.clone());
    tun.start();

    // Start the UDP listener that clients connect to
    let udp_listener =
        udp_listener::WgUdpListener::new(tun_task_tx, peers_by_ip, gateway_client_registry).await?;
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
