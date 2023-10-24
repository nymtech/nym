use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use async_recursion::async_recursion;
use boringtun::{
    noise::{errors::WireGuardError, rate_limiter::RateLimiter, Tunn, TunnResult},
    x25519,
};
use bytes::Bytes;
use log::{debug, error, info, warn};
use rand::RngCore;
use tap::TapFallible;
use tokio::{net::UdpSocket, sync::broadcast, time::timeout};

use crate::{
    active_peers::{peer_event_channel, PeerEventReceiver, PeerEventSender},
    error::WgError,
    event::Event,
    network_table::NetworkTable,
    packet_relayer::PacketRelaySender,
    registered_peers::PeerIdx,
};

const HANDSHAKE_MAX_RATE: u64 = 10;

const MAX_PACKET: usize = 65535;

// We index the tunnels by tag
pub(crate) type PeersByTag = HashMap<u64, PeerEventSender>;

pub struct WireGuardTunnel {
    // Incoming data from the UDP socket received in the main event loop
    peer_rx: PeerEventReceiver,

    // UDP socket used for sending data
    udp: Arc<UdpSocket>,

    // Peer endpoint
    endpoint: Arc<tokio::sync::RwLock<SocketAddr>>,

    // AllowedIPs for this peer
    allowed_ips: NetworkTable<()>,

    // `boringtun` tunnel, used for crypto & WG protocol
    wg_tunnel: Arc<tokio::sync::Mutex<Tunn>>,

    // Signal close
    close_tx: broadcast::Sender<()>,
    close_rx: broadcast::Receiver<()>,

    // Send data to the task that handles sending data through the tun device
    packet_tx: PacketRelaySender,

    tag: u64,
}

impl Drop for WireGuardTunnel {
    fn drop(&mut self) {
        info!("WireGuard tunnel: dropping");
        self.close();
    }
}

impl WireGuardTunnel {
    pub(crate) fn new(
        udp: Arc<UdpSocket>,
        endpoint: SocketAddr,
        static_private: x25519::StaticSecret,
        peer_static_public: x25519::PublicKey,
        index: PeerIdx,
        peer_allowed_ips: ip_network::IpNetwork,
        // rate_limiter: Option<RateLimiter>,
        packet_tx: PacketRelaySender,
    ) -> (Self, PeerEventSender, u64) {
        let local_addr = udp.local_addr().unwrap();
        let peer_addr = udp.peer_addr();
        log::info!("New wg tunnel: endpoint: {endpoint}, local_addr: {local_addr}, peer_addr: {peer_addr:?}");

        let preshared_key = None;
        let persistent_keepalive = None;

        let static_public = x25519::PublicKey::from(&static_private);
        let rate_limiter = Some(Arc::new(RateLimiter::new(
            &static_public,
            HANDSHAKE_MAX_RATE,
        )));

        let wg_tunnel = Arc::new(tokio::sync::Mutex::new(
            Tunn::new(
                static_private,
                peer_static_public,
                preshared_key,
                persistent_keepalive,
                index,
                rate_limiter,
            )
            .expect("failed to create Tunn instance"),
        ));

        // Channels with incoming data that is received by the main event loop
        let (peer_tx, peer_rx) = peer_event_channel();

        // Signal close tunnel
        let (close_tx, close_rx) = broadcast::channel(1);

        let mut allowed_ips = NetworkTable::new();
        allowed_ips.insert(peer_allowed_ips, ());

        let tag = Self::new_tag();

        let tunnel = WireGuardTunnel {
            peer_rx,
            udp,
            endpoint: Arc::new(tokio::sync::RwLock::new(endpoint)),
            allowed_ips,
            wg_tunnel,
            close_tx,
            close_rx,
            packet_tx,
            tag,
        };

        (tunnel, peer_tx, tag)
    }

    fn new_tag() -> u64 {
        // TODO: check for collisions
        rand::thread_rng().next_u64()
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
                            Event::Wg(data) => {
                                let _ = self.consume_wg(&data)
                                    .await
                                    .tap_err(|err| error!("WireGuard tunnel: consume_wg error: {err}"));
                            },
                            Event::WgVerified(data) => {
                                let _ = self.consume_verified_wg(&data)
                                    .await
                                    .tap_err(|err| error!("WireGuard tunnel: consume_verified_wg error: {err}"));
                            }
                            Event::Ip(data) => self.consume_eth(&data).await,
                        }
                    },
                    None => {
                        info!("WireGuard tunnel: incoming UDP stream closed, closing tunnel");
                        break;
                    },
                },
                () = tokio::time::sleep(Duration::from_millis(250)) => {
                    let _ = self.update_wg_timers()
                        .await
                        .map_err(|err| error!("WireGuard tunnel: update_wg_timers error: {err}"));
                },
            }
        }
        info!("WireGuard tunnel ({}): closed", self.endpoint.read().await);
    }

    async fn wg_tunnel_lock(&self) -> Result<tokio::sync::MutexGuard<'_, Tunn>, WgError> {
        timeout(Duration::from_millis(100), self.wg_tunnel.lock())
            .await
            .map_err(|_| WgError::UnableToGetTunnel)
    }

    #[allow(unused)]
    async fn set_endpoint(&self, addr: SocketAddr) {
        if *self.endpoint.read().await != addr {
            log::info!("wg tunnel update endpoint: {addr}");
            *self.endpoint.write().await = addr;
        }
    }

    async fn consume_wg(&mut self, data: &[u8]) -> Result<(), WgError> {
        let mut send_buf = [0u8; MAX_PACKET];
        let mut tunnel = self.wg_tunnel_lock().await?;
        match tunnel.decapsulate(None, data, &mut send_buf) {
            TunnResult::WriteToNetwork(packet) => {
                let endpoint = self.endpoint.read().await;
                log::info!("udp: send {} bytes to {}", packet.len(), *endpoint);
                if let Err(err) = self.udp.send_to(packet, *endpoint).await {
                    error!("Failed to send decapsulation-instructed packet to WireGuard endpoint: {err:?}");
                };
                // Flush pending queue
                loop {
                    let mut send_buf = [0u8; MAX_PACKET];
                    match tunnel.decapsulate(None, &[], &mut send_buf) {
                        TunnResult::WriteToNetwork(packet) => {
                            log::info!("udp: send {} bytes to {}", packet.len(), *endpoint);
                            if let Err(err) = self.udp.send_to(packet, *endpoint).await {
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
            TunnResult::WriteToTunnelV4(packet, addr) => {
                if self.allowed_ips.longest_match(addr).is_some() {
                    self.packet_tx
                        .0
                        .send((self.tag, packet.to_vec()))
                        .await
                        .unwrap();
                } else {
                    warn!("Packet from {addr} not in allowed_ips");
                }
            }
            TunnResult::WriteToTunnelV6(packet, addr) => {
                if self.allowed_ips.longest_match(addr).is_some() {
                    self.packet_tx
                        .0
                        .send((self.tag, packet.to_vec()))
                        .await
                        .unwrap();
                } else {
                    warn!("Packet (v6) from {addr} not in allowed_ips");
                }
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

    async fn consume_verified_wg(&mut self, data: &[u8]) -> Result<(), WgError> {
        // Potentially we could take some shortcuts here in the name of performance, but currently
        // I don't see that the needed functions in boringtun is exposed in the public API.
        // TODO: make sure we don't put double pressure on the rate limiter!
        self.consume_wg(data).await
    }

    async fn consume_eth(&self, data: &Bytes) {
        info!("consume_eth: raw packet size: {}", data.len());
        let encapsulated_packet = self.encapsulate_packet(data).await;
        info!(
            "consume_eth: after encapsulate: {}",
            encapsulated_packet.len()
        );

        let endpoint = self.endpoint.read().await;
        info!("consume_eth: send to {}: {}", *endpoint, data.len());
        self.udp
            .send_to(&encapsulated_packet, *endpoint)
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
                let endpoint = self.endpoint.read().await;
                log::info!("routine: write to network: {}: {}", endpoint, packet.len());
                if let Err(err) = self.udp.send_to(packet, *endpoint).await {
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
                self.handle_routine_tun_result(result).await;
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

pub(crate) fn start_wg_tunnel(
    endpoint: SocketAddr,
    udp: Arc<UdpSocket>,
    static_private: x25519::StaticSecret,
    peer_static_public: x25519::PublicKey,
    peer_index: PeerIdx,
    peer_allowed_ips: ip_network::IpNetwork,
    packet_tx: PacketRelaySender,
) -> (
    tokio::task::JoinHandle<x25519::PublicKey>,
    PeerEventSender,
    u64,
) {
    let (mut tunnel, peer_tx, tag) = WireGuardTunnel::new(
        udp,
        endpoint,
        static_private,
        peer_static_public,
        peer_index,
        peer_allowed_ips,
        packet_tx,
    );
    let join_handle = tokio::spawn(async move {
        tunnel.spin_off().await;
        peer_static_public
    });
    (join_handle, peer_tx, tag)
}
