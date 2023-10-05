use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_recursion::async_recursion;
use boringtun::{
    noise::{errors::WireGuardError, Tunn, TunnResult},
    x25519,
};
use bytes::Bytes;
use etherparse::{InternetSlice, SlicedPacket};
use log::{debug, error, info, warn};
use tap::TapFallible;
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
    time::timeout,
};

use crate::{error::WgError, event::Event};

const MAX_PACKET: usize = 65535;

pub struct WireGuardTunnel {
    // Incoming data from the UDP socket received in the main event loop
    peer_rx: mpsc::UnboundedReceiver<Event>,

    // UDP socket used for sending data
    udp: Arc<UdpSocket>,

    // Peer endpoint
    endpoint: SocketAddr,

    // The source address of the last packet received from the peer
    source_addr: Arc<std::sync::RwLock<Option<std::net::Ipv4Addr>>>,

    // `boringtun` tunnel, used for crypto & WG protocol
    wg_tunnel: Arc<tokio::sync::Mutex<Tunn>>,

    // Signal close
    close_tx: broadcast::Sender<()>,
    close_rx: broadcast::Receiver<()>,

    // Send data to the task that handles sending data through the tun device
    tun_task_tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl Drop for WireGuardTunnel {
    fn drop(&mut self) {
        info!("WireGuard tunnel: dropping");
        self.close();
    }
}

impl WireGuardTunnel {
    pub fn new(
        udp: Arc<UdpSocket>,
        endpoint: SocketAddr,
        static_private: x25519::StaticSecret,
        peer_static_public: x25519::PublicKey,
        tunnel_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> (Self, mpsc::UnboundedSender<Event>) {
        let local_addr = udp.local_addr().unwrap();
        let peer_addr = udp.peer_addr();
        log::info!("New wg tunnel: endpoint: {endpoint}, local_addr: {local_addr}, peer_addr: {peer_addr:?}");

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
        let (peer_tx, peer_rx) = mpsc::unbounded_channel();

        // Signal close tunnel
        let (close_tx, close_rx) = broadcast::channel(1);

        let tunnel = WireGuardTunnel {
            peer_rx,
            udp,
            endpoint,
            source_addr: Default::default(),
            wg_tunnel,
            close_tx,
            close_rx,
            tun_task_tx: tunnel_tx,
        };

        (tunnel, peer_tx)
    }

    fn close(&self) {
        let _ = self.close_tx.send(());
    }

    pub async fn spin_off(&mut self) {
        loop {
            tokio::select! {
                _ = self.close_rx.recv() => {
                    info!("WireGuard tunnel: received msg to close");
                    break;
                },
                packet = self.peer_rx.recv() => match packet {
                    Some(packet) => {
                        info!("event loop: {packet}");
                        match packet {
                            Event::WgPacket(data) => {
                                let _ = self.consume_wg(&data)
                                    .await
                                    .tap_err(|err| error!("WireGuard tunnel: consume_wg error: {err}"));
                            },
                            Event::IpPacket(data) => self.consume_eth(&data).await,
                        }
                    },
                    None => {
                        info!("WireGuard tunnel: incoming UDP stream closed, closing tunnel");
                        break;
                    },
                },
                _ = tokio::time::sleep(Duration::from_millis(250)) => {
                    let _ = self.update_wg_timers()
                        .await
                        .map_err(|err| error!("WireGuard tunnel: update_wg_timers error: {err}"));
                },
            }
        }
        info!("WireGuard tunnel ({}): closed", self.endpoint);
    }

    async fn wg_tunnel_lock(&self) -> Result<tokio::sync::MutexGuard<'_, Tunn>, WgError> {
        timeout(Duration::from_millis(100), self.wg_tunnel.lock())
            .await
            .map_err(|_| WgError::UnableToGetTunnel)
    }

    fn set_source_addr(&self, source_addr: std::net::Ipv4Addr) {
        let to_update = {
            let stored_source_addr = self.source_addr.read().unwrap();
            stored_source_addr
                .map(|sa| sa != source_addr)
                .unwrap_or(true)
        };
        if to_update {
            log::info!("wg tunnel set_source_addr: {source_addr}");
            *self.source_addr.write().unwrap() = Some(source_addr);
        }
    }

    async fn consume_wg(&mut self, data: &[u8]) -> Result<(), WgError> {
        let mut send_buf = [0u8; MAX_PACKET];
        let mut tunnel = self.wg_tunnel_lock().await?;
        match tunnel.decapsulate(None, data, &mut send_buf) {
            TunnResult::WriteToNetwork(packet) => {
                log::info!("udp: send {} bytes to {}", packet.len(), self.endpoint);
                if let Err(err) = self.udp.send_to(packet, self.endpoint).await {
                    error!("Failed to send decapsulation-instructed packet to WireGuard endpoint: {err:?}");
                };
                // Flush pending queue
                loop {
                    let mut send_buf = [0u8; MAX_PACKET];
                    match tunnel.decapsulate(None, &[], &mut send_buf) {
                        TunnResult::WriteToNetwork(packet) => {
                            log::info!("udp: send {} bytes to {}", packet.len(), self.endpoint);
                            if let Err(err) = self.udp.send_to(packet, self.endpoint).await {
                                error!("Failed to send decapsulation-instructed packet to WireGuard endpoint: {err:?}");
                                break;
                            };
                        }
                        _ => {
                            break;
                        }
                    }
                }
            }
            TunnResult::WriteToTunnelV4(packet, _) | TunnResult::WriteToTunnelV6(packet, _) => {
                let headers = SlicedPacket::from_ip(packet).unwrap();
                let (source_addr, _destination_addr) = match headers.ip.unwrap() {
                    InternetSlice::Ipv4(ip, _) => (ip.source_addr(), ip.destination_addr()),
                    InternetSlice::Ipv6(_, _) => unimplemented!(),
                };
                self.set_source_addr(source_addr);
                self.tun_task_tx.send(packet.to_vec()).unwrap();
            }
            TunnResult::Done => {
                debug!("WireGuard: decapsulate done");
            }
            TunnResult::Err(err) => {
                error!("WireGuard: decapsulate error: {err:?}");
            }
        }
        Ok(())
    }

    async fn consume_eth(&self, data: &Bytes) {
        info!("consume_eth: raw packet size: {}", data.len());
        let encapsulated_packet = self.encapsulate_packet(data).await;
        info!(
            "consume_eth: after encapsulate: {}",
            encapsulated_packet.len()
        );

        info!("consume_eth: send to {}: {}", self.endpoint, data.len());
        self.udp
            .send_to(&encapsulated_packet, self.endpoint)
            .await
            .unwrap();
    }

    async fn encapsulate_packet(&self, payload: &[u8]) -> Vec<u8> {
        // TODO: use fixed dst and src buffers that we can reuse
        let len = 148.max(payload.len() + 32);
        let mut dst = vec![0; len];

        let mut wg_tunnel = self.wg_tunnel_lock().await.unwrap();

        match wg_tunnel.encapsulate(payload, &mut dst) {
            TunnResult::WriteToNetwork(packet) => packet.to_vec(),
            unexpected => {
                error!("{:?}", unexpected);
                vec![]
            }
        }
    }

    async fn update_wg_timers(&mut self) -> Result<(), WgError> {
        let mut send_buf = [0u8; MAX_PACKET];
        let mut tun = self.wg_tunnel_lock().await?;
        let tun_result = tun.update_timers(&mut send_buf);
        self.handle_routine_tun_result(tun_result).await;
        Ok(())
    }

    #[async_recursion]
    async fn handle_routine_tun_result<'a: 'async_recursion>(&self, result: TunnResult<'a>) {
        match result {
            TunnResult::WriteToNetwork(packet) => {
                log::info!(
                    "routine: write to network: {}: {}",
                    self.endpoint,
                    packet.len()
                );
                if let Err(err) = self.udp.send_to(packet, self.endpoint).await {
                    error!("routine: failed to send packet: {err:?}");
                };
            }
            TunnResult::Err(WireGuardError::ConnectionExpired) => {
                warn!("Wireguard handshake has expired!");
                // WIP(JON): consider just closing the tunnel here
                let mut buf = vec![0u8; MAX_PACKET];
                let Ok(mut peer) = self.wg_tunnel_lock().await else {
                    warn!("Failed to lock WireGuard peer, closing tunnel");
                    self.close();
                    return;
                };
                peer.format_handshake_initiation(&mut buf[..], false);
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
}

pub fn start_wg_tunnel(
    endpoint: SocketAddr,
    udp: Arc<UdpSocket>,
    static_private: x25519::StaticSecret,
    peer_static_public: x25519::PublicKey,
    tunnel_tx: mpsc::UnboundedSender<Vec<u8>>,
) -> (
    tokio::task::JoinHandle<SocketAddr>,
    mpsc::UnboundedSender<Event>,
) {
    let (mut tunnel, peer_tx) =
        WireGuardTunnel::new(udp, endpoint, static_private, peer_static_public, tunnel_tx);
    let join_handle = tokio::spawn(async move {
        tunnel.spin_off().await;
        endpoint
    });
    (join_handle, peer_tx)
}
