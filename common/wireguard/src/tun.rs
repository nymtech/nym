use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_recursion::async_recursion;
use boringtun::{
    noise::{errors::WireGuardError, Tunn, TunnResult},
    x25519,
};
use bytes::Bytes;
use log::{debug, error, info, warn};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
    time::timeout,
};

use crate::event::Event;

const MAX_PACKET: usize = 65535;

pub struct WireGuardTunnel {
    // Incoming data from the UDP socket received in the main event loop
    udp_rx: mpsc::UnboundedReceiver<Event>,

    // UDP socket used for sending data
    udp: Arc<UdpSocket>,

    // Peer endpoint
    addr: SocketAddr,

    // `boringtun` tunnel, used for crypto & WG protocol
    wg_tunnel: Arc<tokio::sync::Mutex<Tunn>>,

    // Signal close
    close_tx: broadcast::Sender<()>,
    close_rx: broadcast::Receiver<()>,
}

impl Drop for WireGuardTunnel {
    fn drop(&mut self) {
        info!("WireGuard tunnel: dropping");
        self.close();
    }
}

impl WireGuardTunnel {
    fn close(&self) {
        let _ = self.close_tx.send(());
    }

    pub fn new(
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

        // Signal close tunnel
        let (close_tx, mut close_rx) = broadcast::channel(1);

        let tunnel = WireGuardTunnel {
            udp_rx,
            udp,
            addr,
            wg_tunnel,
            close_tx,
            close_rx,
        };

        (tunnel, udp_tx)
    }

    pub async fn spin_off(&mut self) {
        loop {
            tokio::select! {
                _ = self.close_rx.recv() => {
                    info!("WireGuard tunnel: received msg to close");
                    break;
                },
                packet = self.udp_rx.recv() => match packet {
                    Some(packet) => {
                        info!("WireGuard tunnel received: {packet}");
                        match packet {
                            Event::WgPacket(data) => self.consume_wg(&data).await,
                            Event::IpPacket(data) => self.consume_eth(&data).await,
                            _ => {},
                        }
                    },
                    None => {
                        info!("WireGuard tunnel: incoming UDP stream closed, closing tunnel");
                        break;
                    },
                },
                _ = tokio::time::sleep(Duration::from_millis(250)) => {
                    self.update_wg_timers().await;
                },
            }
        }
        info!("WireGuard tunnel ({}): closed", self.addr);
    }

    async fn wg_tunnel_lock(&self) -> tokio::sync::MutexGuard<'_, Tunn> {
        timeout(Duration::from_millis(100), self.wg_tunnel.lock())
            .await
            .unwrap()
    }

    async fn consume_wg(&self, data: &[u8]) {
        let mut send_buf = [0u8; MAX_PACKET];
        let mut peer = self.wg_tunnel_lock().await;
        match peer.decapsulate(None, data, &mut send_buf) {
            TunnResult::WriteToNetwork(packet) => {
                debug!("WireGuard: writing to network");
                if let Err(err) = self.udp.send_to(packet, self.addr).await {
                    error!("Failed to send decapsulation-instructed packet to WireGuard endpoint: {err:?}");
                };
                // Flush pending queue
                loop {
                    let mut send_buf = [0u8; MAX_PACKET];
                    match peer.decapsulate(None, &[], &mut send_buf) {
                        TunnResult::WriteToNetwork(packet) => {
                            if let Err(err) = self.udp.send_to(packet, self.addr).await {
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
                debug!("WireGuard: writing to tunnel");
                info!(
                    "WireGuard endpoint sent IP packet of {} bytes (not yet implemented)",
                    packet.len()
                );
                // TODO
            }
            TunnResult::Done => {
                debug!("WireGuard: decapsulate done");
            }
            TunnResult::Err(err) => {
                error!("WireGuard: decapsulate error: {err:?}");
            }
        }
    }

    async fn consume_eth(&self, _data: &Bytes) {
        info!("WireGuard tunnel: consume_eth");
        todo!();
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
}
