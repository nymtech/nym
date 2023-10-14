use std::{net::SocketAddr, sync::Arc, time::Duration};

use boringtun::{
    noise::{self, handshake::parse_handshake_anon, rate_limiter::RateLimiter, TunnResult},
    x25519,
};
use dashmap::DashMap;
use futures::StreamExt;
use log::error;
use nym_task::TaskClient;
use tap::TapFallible;
use tokio::{
    net::UdpSocket,
    sync::mpsc::{self},
};

use crate::{
    event::Event,
    network_table::NetworkTable,
    registered_peers::{RegisteredPeer, RegisteredPeers},
    setup::{self, WG_ADDRESS, WG_PORT},
    TunTaskTx,
};

const MAX_PACKET: usize = 65535;

// Registered peers
pub(crate) type PeersByIp = NetworkTable<mpsc::UnboundedSender<Event>>;

// Active peers
pub(crate) type ActivePeers = DashMap<x25519::PublicKey, mpsc::UnboundedSender<Event>>;
pub(crate) type PeersByAddr = DashMap<SocketAddr, mpsc::UnboundedSender<Event>>;

async fn add_test_peer(registered_peers: &mut RegisteredPeers) {
    let peer_static_public = setup::peer_static_public_key();
    let peer_index = 0;
    let peer_allowed_ips = setup::peer_allowed_ips();
    let test_peer = Arc::new(tokio::sync::Mutex::new(RegisteredPeer {
        public_key: peer_static_public,
        index: peer_index,
        allowed_ips: peer_allowed_ips,
    }));
    registered_peers.insert(peer_static_public, test_peer).await;
}

pub(crate) async fn start_udp_listener(
    tun_task_tx: TunTaskTx,
    peers_by_ip: Arc<std::sync::Mutex<PeersByIp>>,
    mut task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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

    tokio::spawn(async move {
        // The set of active tunnels indexed by the peer's address
        let active_peers = Arc::new(ActivePeers::new());
        let active_peers_by_addr = PeersByAddr::new();
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
                    rate_limiter.reset_count();
                },
                // Handle tunnel closing
                Some(public_key) = active_peers_task_handles.next() => {
                    match public_key {
                        Ok(public_key) => {
                            log::info!("Removing peer: {public_key:?}");
                            active_peers.remove(&public_key);
                            log::warn!("TODO: remove from peers_by_ip?");
                            log::warn!("TODO: remove from peers_by_addr");
                        }
                        Err(err) => {
                            error!("WireGuard UDP listener: error receiving shutdown from peer: {err}");
                        }
                    }
                },
                // Handle incoming packets
                Ok((len, addr)) = udp.recv_from(&mut buf) => {
                    log::trace!("udp: received {} bytes from {}", len, addr);

                    // If this addr has already been encountered, send directly to tunnel
                    // TODO: optimization opportunity to instead create a connected UDP socket
                    // inside the wg tunnel, where you can recv the data directly.
                    if let Some(peer_tx) = active_peers_by_addr.get(&addr) {
                        log::info!("udp: received {len} bytes from {addr} from known peer");
                        peer_tx
                            .send(Event::Wg(buf[..len].to_vec().into()))
                            .tap_err(|e| log::error!("{e}"))
                            .ok();
                    }

                    // Verify the incoming packet
                    let verified_packet = match rate_limiter.verify_packet(Some(addr.ip()), &buf[..len], &mut dst_buf) {
                        Ok(packet) => packet,
                        Err(TunnResult::WriteToNetwork(cookie)) => {
                            log::info!("Send back cookie to: {addr}");
                            udp.send_to(cookie, addr).await.tap_err(|e| log::error!("{e}")).ok();
                            continue;
                        }
                        Err(err) => {
                            log::warn!("{err:?}");
                            continue;
                        }
                    };

                    // Check if this is a registered peer, if not, just skip
                    let registered_peer = {
                        let reg_peer = match verified_packet {
                            noise::Packet::HandshakeInit(ref packet) => {
                                let Ok(handshake) = parse_handshake_anon(&static_private, &static_public, packet) else {
                                    log::warn!("Handshake failed: {addr}");
                                    continue;
                                };
                                registered_peers.get_by_key(&x25519::PublicKey::from(handshake.peer_static_public))
                            },
                            noise::Packet::HandshakeResponse(packet) => {
                                let peer_idx = packet.receiver_idx >> 8;
                                registered_peers.get_by_idx(peer_idx)
                            },
                            noise::Packet::PacketCookieReply(packet) => {
                                let peer_idx = packet.receiver_idx >> 8;
                                registered_peers.get_by_idx(peer_idx)
                            },
                            noise::Packet::PacketData(packet) => {
                                let peer_idx = packet.receiver_idx >> 8;
                                registered_peers.get_by_idx(peer_idx)
                            },
                        };

                        match reg_peer {
                            Some(reg_peer) => reg_peer.lock().await,
                            None => {
                                log::warn!("Peer not registered: {addr}");
                                continue;
                            }
                        }
                    };

                    // Look up if the peer is already connected
                    if let Some(peer_tx) = active_peers.get_mut(&registered_peer.public_key) {
                        // We found the peer as connected, even though the addr was not known
                        log::info!("udp: received {len} bytes from {addr} which is a known peer with unknown addr");
                        peer_tx.send(Event::WgVerified(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .ok();
                    } else {
                        // If it isn't, start a new tunnel
                        log::info!("udp: received {len} bytes from {addr} from unknown peer, starting tunnel");
                        // NOTE: we are NOT passing in the existing rate_limiter. Re-visit this
                        // choice later.
                        log::warn!("Creating new rate limiter, consider re-using");
                        let (join_handle, peer_tx) = crate::wg_tunnel::start_wg_tunnel(
                            addr,
                            udp.clone(),
                            static_private.clone(),
                            registered_peer.public_key,
                            registered_peer.index,
                            registered_peer.allowed_ips,
                            tun_task_tx.clone(),
                        );

                        peers_by_ip.lock().unwrap().insert(registered_peer.allowed_ips, peer_tx.clone());
                        active_peers_by_addr.insert(addr, peer_tx.clone());

                        peer_tx.send(Event::Wg(buf[..len].to_vec().into()))
                            .tap_err(|e| log::error!("{e}"))
                            .ok();

                        log::info!("Adding peer: {addr}");
                        active_peers.insert(registered_peer.public_key, peer_tx);
                        active_peers_task_handles.push(join_handle);
                    }
                },
            }
        }
        log::info!("WireGuard listener: shutting down");
    });

    Ok(())
}
