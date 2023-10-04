use nym_task::TaskClient;

pub use error::WgError;

mod error;
#[cfg(target_os = "linux")]
mod event;
#[cfg(target_os = "linux")]
mod setup;
#[cfg(target_os = "linux")]
mod tun;
#[cfg(target_os = "linux")]
mod tun_device;
#[cfg(target_os = "linux")]
mod udp_listener;

#[cfg(target_os = "linux")]
type ActivePeers =
    dashmap::DashMap<std::net::SocketAddr, tokio::sync::mpsc::UnboundedSender<crate::event::Event>>;

#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // The set of active tunnels indexed by the peer's address
    let active_peers = std::sync::Arc::new(ActivePeers::new());

    // Start the tun device that is used to relay traffic outbound
    let tun_task_tx = tun_device::start_tun_device(active_peers.clone());

    // Start the UDP listener that clients connect to
    udp_listener::start_udp_listener(tun_task_tx, active_peers, task_client).await?;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard(
    _task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    todo!("WireGuard is currently only supported on Linux")
}
