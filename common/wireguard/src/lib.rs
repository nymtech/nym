#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

mod error;
mod event;
mod network_table;
mod platform;
mod setup;
mod udp_listener;
mod wg_tunnel;

// Currently the module related to setting up the virtual network device is platform specific.
#[cfg(target_os = "linux")]
use platform::linux::tun_device;

#[derive(Clone)]
struct TunTaskTx(tokio::sync::mpsc::UnboundedSender<Vec<u8>>);

impl TunTaskTx {
    fn send(&self, packet: Vec<u8>) -> Result<(), tokio::sync::mpsc::error::SendError<Vec<u8>>> {
        self.0.send(packet)
    }
}

#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    task_client: nym_task::TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use std::sync::Arc;

    // The set of active tunnels indexed by the peer's address
    let active_peers = Arc::new(udp_listener::ActivePeers::new());
    let peers_by_ip = Arc::new(std::sync::Mutex::new(network_table::NetworkTable::new()));

    // Start the tun device that is used to relay traffic outbound
    let tun_task_tx = tun_device::start_tun_device(peers_by_ip.clone());

    // Start the UDP listener that clients connect to
    udp_listener::start_udp_listener(tun_task_tx, active_peers, peers_by_ip, task_client).await?;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard(
    _task_client: nym_task::TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    todo!("WireGuard is currently only supported on Linux")
}
