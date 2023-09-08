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
use tokio::{
    net::UdpSocket,
    sync::{mpsc, oneshot},
};

//const WG_ADDRESS = "0.0.0.0:51820";
const WG_ADDRESS: &str = "0.0.0.0:51822";

const PRIVATE_KEY: &str = "AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=";

const PEERS: &[&str; 1] = &["mxV/mw7WZTe+0Msa0kvJHMHERDA/cSskiZWQce+TdEs="];

pub async fn start_wg_listener(
    mut task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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
                        let (mut tunnel, peer_tx) = WireGuardTunnel::new();
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

struct WireGuardTunnel {
    // Incoming data from the UDP socket
    udp_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,

    // `boringtun` tunnel, used for crypto & WG protocol
    wg_tunnel: boringtun::noise::Tunn,
}

impl WireGuardTunnel {
    fn new() -> (Self, mpsc::UnboundedSender<Event>) {
        let (udp_tx, udp_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                udp_rx,
                wg_tunnel: todo!(),
            },
            udp_tx,
        )
    }

    async fn spin_off(&mut self) {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(2000)) => {
                    log::info!("WireGuard tunnel: shutting down");
                    break;
                }
                packet = self.udp_rx.recv() => {
                    match packet {
                        Some(packet) => {
                            log::info!("WireGuard tunnel received: {packet}");
                            match packet {
                                Event::IpPacket(data) => self.consume_eth(&data).await,
                                Event::WgPacket(data) => self.consume_wg(&data).await,
                                _ => {},
                            }
                        }
                        None => log::error!("none"),
                    }
                }
            }
        }
    }

    async fn consume_eth(&self, data: &Bytes) {
        log::info!("WireGuard tunnel: consume_eth");
        todo!();
    }

    async fn consume_wg(&self, data: &Bytes) {
        log::info!("WireGuard tunnel: consume_wg");
        todo!();
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
