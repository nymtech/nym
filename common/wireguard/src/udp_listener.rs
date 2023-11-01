use std::{net::SocketAddr, sync::Arc, time::Duration};

use boringtun::{
    noise::{self, handshake::parse_handshake_anon, rate_limiter::RateLimiter, TunnResult},
    x25519,
};
use futures::StreamExt;
use log::error;
use nym_task::TaskClient;
use nym_wireguard_types::{registration::GatewayClientRegistry, PeerPublicKey, WG_PORT};
use tap::TapFallible;
use tokio::{net::UdpSocket, sync::Mutex};

use crate::{
    active_peers::{ActivePeers, PeerEventSender},
    error::WgError,
    event::Event,
    network_table::NetworkTable,
    packet_relayer::PacketRelaySender,
    registered_peers::{RegisteredPeer, RegisteredPeers},
    setup::{self, WG_ADDRESS},
    wg_tunnel::PeersByTag,
};

const MAX_PACKET: usize = 65535;

// Registered peers
pub(crate) type PeersByIp = NetworkTable<PeerEventSender>;

async fn add_test_peer(registered_peers: &mut RegisteredPeers) {
    let peer_static_public = PeerPublicKey::new(setup::peer_static_public_key());
    let peer_index = 0;
    let peer_allowed_ips = setup::peer_allowed_ips();
    let test_peer = Arc::new(tokio::sync::Mutex::new(RegisteredPeer {
        public_key: peer_static_public,
        index: peer_index,
        allowed_ips: peer_allowed_ips,
    }));
    registered_peers.insert(test_peer).await;
}

pub struct WgUdpListener {
    // Our private key
    static_private: x25519::StaticSecret,

    // Our public key
    static_public: x25519::PublicKey,

    // The list of registered peers that we allow
    registered_peers: RegisteredPeers,

    // The routing table, as defined by wireguard
    peers_by_ip: Arc<tokio::sync::Mutex<PeersByIp>>,

    // ... or alternatively we can map peers by their tag
    peers_by_tag: Arc<tokio::sync::Mutex<PeersByTag>>,

    // The UDP socket to the peer
    udp: Arc<UdpSocket>,

    // Send data to the TUN device for sending
    packet_tx: PacketRelaySender,

    // Wireguard rate limiter
    rate_limiter: RateLimiter,

    gateway_client_registry: Arc<GatewayClientRegistry>,
}

impl WgUdpListener {
    pub async fn new(
        packet_tx: PacketRelaySender,
        peers_by_ip: Arc<tokio::sync::Mutex<PeersByIp>>,
        peers_by_tag: Arc<tokio::sync::Mutex<PeersByTag>>,
        gateway_client_registry: Arc<GatewayClientRegistry>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let wg_address = SocketAddr::new(WG_ADDRESS.parse().unwrap(), WG_PORT);
        log::info!("Starting wireguard UDP listener on {wg_address}");
        let udp = Arc::new(UdpSocket::bind(wg_address).await?);

        // Setup our own keys
        let static_private = setup::server_static_private_key();
        let static_public = x25519::PublicKey::from(&static_private);
        let handshake_max_rate = 100u64;
        let rate_limiter = RateLimiter::new(&static_public, handshake_max_rate);

        // Create a test peer for dev
        let mut registered_peers = RegisteredPeers::default();
        add_test_peer(&mut registered_peers).await;

        Ok(Self {
            static_private,
            static_public,
            registered_peers,
            peers_by_ip,
            peers_by_tag,
            udp,
            packet_tx,
            rate_limiter,
            gateway_client_registry,
        })
    }

    pub async fn run(mut self, mut task_client: TaskClient) {
        // The set of active tunnels
        let active_peers = ActivePeers::default();
        // Each tunnel is run in its own task, and the task handle is stored here so we can remove
        // it from `active_peers` when the tunnel is closed
        let mut active_peers_task_handles = futures::stream::FuturesUnordered::new();

        let mut buf = [0u8; MAX_PACKET];
        let mut dst_buf = [0u8; MAX_PACKET];

        while !task_client.is_shutdown() {
            tokio::select! {
                () = task_client.recv() => {
                    log::trace!("WireGuard UDP listener: received shutdown");
                    break;
                }
                // Reset the rate limiter every 1 sec
                () = tokio::time::sleep(Duration::from_secs(1)) => {
                    self.rate_limiter.reset_count();
                },
                // Handle tunnel closing
                Some(public_key) = active_peers_task_handles.next() => {
                    match public_key {
                        Ok(public_key) => {
                            active_peers.remove(&public_key);
                        }
                        Err(err) => {
                            error!("WireGuard UDP listener: error receiving shutdown from peer: {err}");
                        }
                    }
                },
                // Handle incoming packets
                Ok((len, addr)) = self.udp.recv_from(&mut buf) => {
                    log::trace!("udp: received {} bytes from {}", len, addr);

                    // If this addr has already been encountered, send directly to tunnel
                    // TODO: optimization opportunity to instead create a connected UDP socket
                    // inside the wg tunnel, where you can recv the data directly.
                    if let Some(peer_tx) = active_peers.get_by_addr(&addr) {
                        log::info!("udp: received {len} bytes from {addr} from known peer");
                        peer_tx
                            .send(Event::Wg(buf[..len].to_vec().into()))
                            .await
                            .tap_err(|e| log::error!("{e}"))
                            .ok();
                        continue;
                    }

                    // Verify the incoming packet
                    let verified_packet = match self.rate_limiter.verify_packet(Some(addr.ip()), &buf[..len], &mut dst_buf) {
                        Ok(packet) => packet,
                        Err(TunnResult::WriteToNetwork(cookie)) => {
                            log::info!("Send back cookie to: {addr}");
                            self.udp.send_to(cookie, addr).await.tap_err(|e| log::error!("{e}")).ok();
                            continue;
                        }
                        Err(err) => {
                            log::warn!("{err:?}");
                            continue;
                        }
                    };

                    // Check if this is a registered peer, if not, just skip
                    let registered_peer = match parse_peer(
                        verified_packet,
                        &mut self.registered_peers,
                        &self.static_private,
                        &self.static_public,
                        Arc::clone(&self.gateway_client_registry),
                    ).await {
                        Ok(Some(peer)) => peer.lock().await,
                        Ok(None) => {
                            log::warn!("Peer not registered: {addr}");
                            continue;
                        }
                        Err(err) => {
                            log::error!("{err}");
                            continue;
                        },
                    };

                    // Look up if the peer is already connected
                    if let Some(peer_tx) = active_peers.get_by_key_mut(&registered_peer.public_key) {
                        // We found the peer as connected, even though the addr was not known
                        log::info!("udp: received {len} bytes from {addr} which is a known peer with unknown addr");
                        peer_tx.send(Event::WgVerified(buf[..len].to_vec().into()))
                            .await
                            .tap_err(|err| log::error!("{err}"))
                            .ok();
                    } else {
                        // If it isn't, start a new tunnel
                        log::info!("udp: received {len} bytes from {addr} from unknown peer, starting tunnel");
                        // NOTE: we are NOT passing in the existing rate_limiter. Re-visit this
                        // choice later.
                        log::warn!("Creating new rate limiter, consider re-using?");
                        let (join_handle, peer_tx, tag) = crate::wg_tunnel::start_wg_tunnel(
                            addr,
                            self.udp.clone(),
                            self.static_private.clone(),
                            *registered_peer.public_key,
                            registered_peer.index,
                            registered_peer.allowed_ips,
                            // self.tun_task_tx.clone(),
                            self.packet_tx.clone(),
                        );

                        self.peers_by_ip.lock().await.insert(registered_peer.allowed_ips, peer_tx.clone());
                        self.peers_by_tag.lock().await.insert(tag, peer_tx.clone());

                        peer_tx.send(Event::Wg(buf[..len].to_vec().into()))
                            .await
                            .tap_err(|e| log::error!("{e}"))
                            .ok();

                        log::info!("Adding peer: {:?}: {addr}", registered_peer.public_key);
                        active_peers.insert(*registered_peer.public_key, addr, peer_tx);
                        active_peers_task_handles.push(join_handle);
                    }
                },
            }
        }
        log::info!("WireGuard listener: shutting down");
    }

    pub fn start(self, task_client: TaskClient) {
        tokio::spawn(async move { self.run(task_client).await });
    }
}

async fn parse_peer<'a>(
    verified_packet: noise::Packet<'a>,
    registered_peers: &'a mut RegisteredPeers,
    static_private: &x25519::StaticSecret,
    static_public: &x25519::PublicKey,
    gateway_client_registry: Arc<GatewayClientRegistry>,
) -> Result<Option<&'a Arc<tokio::sync::Mutex<RegisteredPeer>>>, WgError> {
    let registered_peer = match verified_packet {
        noise::Packet::HandshakeInit(ref packet) => {
            let Ok(handshake) = parse_handshake_anon(static_private, static_public, packet) else {
                return Err(WgError::HandshakeFailed);
            };
            let peer_public_key =
                PeerPublicKey::new(x25519::PublicKey::from(handshake.peer_static_public));

            let already_registered = registered_peers.contains_key(&peer_public_key);

            if already_registered {
                registered_peers.get_by_key(&peer_public_key)
            } else if gateway_client_registry.contains_key(&peer_public_key) {
                let peer_idx = registered_peers.next_idx();
                let peer = Arc::new(Mutex::new(RegisteredPeer {
                    public_key: peer_public_key,
                    index: peer_idx,
                    allowed_ips: setup::peer_allowed_ips(),
                }));
                registered_peers.insert(peer).await;
                registered_peers.get_by_key(&peer_public_key)
            } else {
                None
            }
        }
        noise::Packet::HandshakeResponse(packet) => {
            let peer_idx = packet.receiver_idx >> 8;
            registered_peers.get_by_idx(peer_idx)
        }
        noise::Packet::PacketCookieReply(packet) => {
            let peer_idx = packet.receiver_idx >> 8;
            registered_peers.get_by_idx(peer_idx)
        }
        noise::Packet::PacketData(packet) => {
            let peer_idx = packet.receiver_idx >> 8;
            registered_peers.get_by_idx(peer_idx)
        }
    };
    Ok(registered_peer)
}
