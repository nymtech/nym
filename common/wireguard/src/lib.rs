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

use nym_wireguard_types::registration::GatewayClientRegistry;
use std::sync::Arc;

// Currently the module related to setting up the virtual network device is platform specific.
#[cfg(target_os = "linux")]
use platform::linux::tun_device;

type TunTaskPayload = (u64, Vec<u8>);

#[derive(Clone)]
pub struct TunTaskTx(tokio::sync::mpsc::UnboundedSender<TunTaskPayload>);

impl TunTaskTx {
    fn send(
        &self,
        data: TunTaskPayload,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<TunTaskPayload>> {
        self.0.send(data)
    }
}

pub struct TunTaskRx(tokio::sync::mpsc::UnboundedReceiver<TunTaskPayload>);

impl TunTaskRx {
    async fn recv(&mut self) -> Option<TunTaskPayload> {
        self.0.recv().await
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
    // We can either index peers by their IP like standard wireguard
    let peers_by_ip = Arc::new(std::sync::Mutex::new(network_table::NetworkTable::new()));

    // ... or by their tunnel tag, which is a random number assigned to them
    let peers_by_tag = Arc::new(std::sync::Mutex::new(wg_tunnel::PeersByTag::new()));

    // Start the tun device that is used to relay traffic outbound
    let (tun, tun_task_tx) = tun_device::TunDevice::new(peers_by_ip.clone(), peers_by_tag.clone());
    tun.start();

    // Start the UDP listener that clients connect to
    let udp_listener = udp_listener::WgUdpListener::new(
        tun_task_tx,
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
