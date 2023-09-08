use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use async_recursion::async_recursion;
use base64::{engine::general_purpose, Engine as _};
use boringtun::{
    noise::{errors::WireGuardError, Tunn, TunnResult},
    x25519,
};
use bytes::Bytes;
use futures::StreamExt;
use log::{error, info, warn};
use nym_task::TaskClient;
use tap::TapFallible;
use tokio::{net::UdpSocket, sync::mpsc, task::JoinHandle, time::timeout};

//const WG_ADDRESS = "0.0.0.0:51820";
const WG_ADDRESS: &str = "0.0.0.0:51822";

const PRIVATE_KEY: &str = "AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=";

const PEERS: &[&str; 1] = &["mxV/mw7WZTe+0Msa0kvJHMHERDA/cSskiZWQce+TdEs="];

const MAX_PACKET: usize = 65535;

fn init_static_dev_keys() -> (x25519::StaticSecret, x25519::PublicKey) {
    // TODO: this is a temporary solution for development
    let static_private_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(PRIVATE_KEY)
        .unwrap()
        .try_into()
        .unwrap();
    let static_private = x25519::StaticSecret::try_from(static_private_bytes).unwrap();
    let static_public = x25519::PublicKey::from(&static_private);
    info!(
        "wg public key: {}",
        general_purpose::STANDARD.encode(static_public)
    );

    // TODO: A single static public key is used for all peers during development
    let peer_static_public_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(PEERS[0])
        .unwrap()
        .try_into()
        .unwrap();
    let peer_static_public = x25519::PublicKey::try_from(peer_static_public_bytes).unwrap();

    (static_private, peer_static_public)
}

pub async fn start_wg_listener(
    mut task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!("Starting wireguard listener on {}", WG_ADDRESS);
    let udp_socket = Arc::new(UdpSocket::bind(WG_ADDRESS).await?);

    // Setup some static keys for development
    let (static_private, peer_static_public) = init_static_dev_keys();

    tokio::spawn(async move {
        let mut active_peers: HashMap<SocketAddr, mpsc::UnboundedSender<Event>> = HashMap::new();
        let mut active_peers_task_handles = futures::stream::FuturesUnordered::new();
        let mut buf = [0u8; 1024];

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::trace!("WireGuard listener: received shutdown");
                    break;
                }
                // Handle tunnel closing
                Some(addr) = active_peers_task_handles.next() => {
                    match addr {
                        Ok(addr) => {
                            info!("WireGuard listener: received shutdown from {addr:?}");
                            active_peers.remove(&addr);
                        }
                        Err(err) => {
                            error!("WireGuard listener: error receiving shutdown from peer: {err}");
                        }
                    }
                }
                // Handle incoming packets
                Ok((len, addr)) = udp_socket.recv_from(&mut buf) => {
                    log::info!("Received {} bytes from {}", len, addr);

                    if let Some(peer_tx) = active_peers.get_mut(&addr) {
                        log::info!("WireGuard listener: received packet from known peer");
                        peer_tx.send(Event::WgPacket(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();
                    } else {
                        log::info!("WireGuard listener: received packet from unknown peer");
                        let (join_handle, peer_tx) = start_wg_tunnel(
                            addr,
                            udp_socket.clone(),
                            static_private.clone(),
                            peer_static_public
                        );
                        peer_tx.send(Event::WgPacket(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();

                        active_peers.insert(addr, peer_tx);
                        active_peers_task_handles.push(join_handle);
                    }
                }
            }
        }
        log::info!("WireGuard listener: shutting down");
    });

    Ok(())
}

fn start_wg_tunnel(
    addr: SocketAddr,
    udp: Arc<UdpSocket>,
    static_private: x25519::StaticSecret,
    peer_static_public: x25519::PublicKey,
) -> (JoinHandle<SocketAddr>, mpsc::UnboundedSender<Event>) {
    let (mut tunnel, peer_tx) = WireGuardTunnel::new(udp, addr, static_private, peer_static_public);
    let join_handle = tokio::spawn(async move {
        tunnel.spin_off().await;
        addr
    });
    (join_handle, peer_tx)
}

struct WireGuardTunnel {
    // Incoming data from the UDP socket received in the main event loop
    udp_rx: mpsc::UnboundedReceiver<Event>,

    // UDP socket used for sending data
    udp: Arc<UdpSocket>,

    // Peer endpoint
    addr: SocketAddr,

    // `boringtun` tunnel, used for crypto & WG protocol
    wg_tunnel: Arc<tokio::sync::Mutex<Tunn>>,
}

impl WireGuardTunnel {
    fn new(
        udp: Arc<UdpSocket>,
        addr: SocketAddr,
        static_private: x25519::StaticSecret,
        peer_static_public: x25519::PublicKey,
    ) -> (Self, mpsc::UnboundedSender<Event>) {
        let preshared_key = None;
        let persistent_keepalive = None;
        let index = 0;
        let rate_limiter = None;

        let wg_tunnel = Arc::new(tokio::sync::Mutex::new(
            Tunn::new(
                static_private,
                peer_static_public,
                preshared_key,
                persistent_keepalive,
                index,
                rate_limiter,
            )
            .unwrap(),
        ));

        // Channels with incoming data that is received by the main event loop
        let (udp_tx, udp_rx) = mpsc::unbounded_channel();

        let tunnel = WireGuardTunnel {
            udp_rx,
            udp,
            addr,
            wg_tunnel,
        };

        (tunnel, udp_tx)
    }

    async fn spin_off(&mut self) {
        loop {
            tokio::select! {
                // WIP(JON): during dev only
                _ = tokio::time::sleep(Duration::from_millis(2000)) => {
                    log::info!("WireGuard tunnel: shutting down");
                    break;
                },
                packet = self.udp_rx.recv() => match packet {
                    Some(packet) => {
                        log::info!("WireGuard tunnel received: {packet}");
                        match packet {
                            Event::IpPacket(data) => self.consume_eth(&data).await,
                            Event::WgPacket(data) => self.consume_wg(&data).await,
                            _ => {},
                        }
                    },
                    None => log::error!("none"),
                },
                _ = tokio::time::sleep(Duration::from_millis(250)) => {
                    self.update_wg_timers().await;
                },
            }
        }
    }

    async fn wg_tunnel_lock(&self) -> tokio::sync::MutexGuard<'_, Tunn> {
        timeout(Duration::from_millis(100), self.wg_tunnel.lock())
            .await
            .unwrap()
    }

    async fn update_wg_timers(&mut self) {
        let mut send_buf = [0u8; MAX_PACKET];
        let mut tun = self.wg_tunnel_lock().await;
        let tun_result = tun.update_timers(&mut send_buf);
        self.handle_routine_tun_result(tun_result).await;
    }

    #[async_recursion]
    async fn handle_routine_tun_result<'a: 'async_recursion>(&self, result: TunnResult<'a>) {
        match result {
            TunnResult::WriteToNetwork(packet) => {
                info!(
                    "Sending routine packet of {} bytes to WireGuard endpoint",
                    packet.len()
                );
                if let Err(err) = self.udp.send_to(packet, self.addr).await {
                    error!("Failed to send routine packet to WireGuard endpoint: {err:?}",);
                };
            }
            TunnResult::Err(WireGuardError::ConnectionExpired) => {
                warn!("Wireguard handshake has expired!");
                let mut buf = vec![0u8; MAX_PACKET];
                let result = self
                    .wg_tunnel_lock()
                    .await
                    .format_handshake_initiation(&mut buf[..], false);
                self.handle_routine_tun_result(result).await
            }
            TunnResult::Err(err) => {
                error!("Failed to prepare routine packet for WireGuard endpoint: {err:?}");
            }
            TunnResult::Done => {}
            other => {
                warn!("Unexpected WireGuard routine task state: {other:?}");
            }
        };
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
