use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    sync::Arc,
    time::Duration,
};

use bytes::Bytes;
use futures::StreamExt;
use nym_task::TaskClient;
use tap::TapFallible;
use tokio::{net::UdpSocket, sync::oneshot};

//const WG_ADDRESS = "0.0.0.0:51820";
const WG_ADDRESS: &str = "0.0.0.0:51822";

const PRIVATE_KEY: &str = "AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=";

const PEERS: &[&str; 1] = &["mxV/mw7WZTe+0Msa0kvJHMHERDA/cSskiZWQce+TdEs="];

pub async fn start_wg_listener(
    mut task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting wireguard listener on {}", WG_ADDRESS);
    let udp4_socket = Arc::new(UdpSocket::bind(WG_ADDRESS).await?);

    tokio::spawn(async move {
        let mut active_peers: HashMap<
            std::net::SocketAddr,
            tokio::sync::mpsc::UnboundedSender<Event>,
        > = HashMap::new();
        let mut active_peers_rx = futures::stream::FuturesUnordered::new();
        let mut buf = [0u8; 1024];

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::trace!("WireGuard listener: received shutdown");
                    break;
                }
                // Handle tunnel closing
                Some(addr) = active_peers_rx.next() => {
                    match addr {
                        Ok(addr) => {
                            log::info!("WireGuard listener: received shutdown from {:?}", addr);
                            active_peers.remove(&addr);
                        }
                        Err(err) => {
                            log::error!("WireGuard listener: error receiving shutdown from peer: {}", err);
                        }
                    }
                }
                // Handle incoming packets
                Ok((len, addr)) = udp4_socket.recv_from(&mut buf) => {
                    log::info!("Received {} bytes from {}", len, addr);

                    if let Some(peer_tx) = active_peers.get_mut(&addr) {
                        log::info!("WireGuard listener: received packet from known peer");
                        peer_tx.send(Event::WgPacket(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();
                    } else {
                        log::info!("WireGuard listener: received packet from unknown peer");

                        // First setup new tunnel
                        let (peer_tx, peer_rx) = tokio::sync::mpsc::unbounded_channel();
                        let tunnel = WireGuardTunnel::new(peer_rx);
                        let join_handle = tokio::spawn(async move {
                            tunnel.spin_off().await;
                            addr
                        });

                        peer_tx.send(Event::WgPacket(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();

                        active_peers.insert(addr, peer_tx);
                        active_peers_rx.push(join_handle);
                    }
                }
            }
        }
        log::info!("WireGuard listener: shutting down");
    });

    Ok(())
}

struct WireGuardTunnel {}

impl WireGuardTunnel {
    fn new(peer_rx: tokio::sync::mpsc::UnboundedReceiver<Event>) -> Self {
        Self {}
    }

    async fn spin_off(self) {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(2000)) => {
                    log::info!("WireGuard tunnel: shutting down");
                    break;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Dumb event with no data.
    Dumb,
    /// IP packet received from the WireGuard tunnel that should be passed through to the corresponding virtual device/internet.
    /// Original implementation also has protocol here since it understands it, but we'll have to infer it downstream
    WgPacket(Bytes),
    /// IP packet to be sent through the WireGuard tunnel as crafted by the virtual device.
    IpPacket(Bytes),
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Dumb => {
                write!(f, "Dumb{{}}")
            }
            Event::WgPacket(data) => {
                let size = data.len();
                write!(f, "WgPacket{{ size={size} }}")
            }
            Event::IpPacket(data) => {
                let size = data.len();
                write!(f, "IpPacket{{ size={size} }}")
            }
        }
    }
}
