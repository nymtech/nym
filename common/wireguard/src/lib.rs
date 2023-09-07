use std::sync::Arc;

use nym_task::TaskClient;
use tokio::net::UdpSocket;

//const WG_ADDRESS = "0.0.0.0:51820";
const WG_ADDRESS: &str = "0.0.0.0:51822";

pub async fn start_wg_listener(mut task_client: TaskClient) -> Result<(), Box<dyn std::error::Error>>{
    log::info!("Starting Wireguard listener on {}", WG_ADDRESS);

    let udp4_socket = Arc::new(UdpSocket::bind(WG_ADDRESS).await?);

    let mut buf = [0u8; 1024];
    tokio::spawn(async move {
        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::trace!("WireGuard listener: received shutdown");
                }
                Ok((len, addr)) = udp4_socket.recv_from(&mut buf) => {
                    log::info!("Received {} bytes from {}", len, addr);
                    handle_incoming_packet().await;
                }
            }
        }
        log::info!("WireGuard listener: shutting down");
    });

    Ok(())
}

async fn handle_incoming_packet(packet: &[u8]) {

}

struct WireGuardTunnel {
}
