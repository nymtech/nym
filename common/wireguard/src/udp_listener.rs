use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use boringtun::{
    noise::{self, handshake::parse_handshake_anon, rate_limiter::RateLimiter, TunnResult},
    x25519,
};
use dashmap::DashMap;
use futures::StreamExt;
use ip_network::IpNetwork;
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
    setup::{self, WG_ADDRESS, WG_PORT},
    TunTaskTx,
};

const MAX_PACKET: usize = 65535;

// Registered peers
pub(crate) type PeerIdx = u32;
pub(crate) type PeersByIp = NetworkTable<mpsc::UnboundedSender<Event>>;

// Active peers
pub(crate) type ActivePeers = DashMap<x25519::PublicKey, mpsc::UnboundedSender<Event>>;
pub(crate) type PeersByAddr = DashMap<SocketAddr, mpsc::UnboundedSender<Event>>;

#[derive(Debug)]
struct RegisteredPeer {
    public_key: x25519::PublicKey,
    allowed_ips: IpNetwork,
    // endpoint: SocketAddr,
}

pub(crate) async fn start_udp_listener(
    tun_task_tx: TunTaskTx,
    peers_by_ip: Arc<std::sync::Mutex<PeersByIp>>,
    mut task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let wg_address = SocketAddr::new(WG_ADDRESS.parse().unwrap(), WG_PORT);
    log::info!("Starting wireguard UDP listener on {wg_address}");
    let udp_socket = Arc::new(UdpSocket::bind(wg_address).await?);

    // Setup our own keys
    let static_private = setup::server_static_private_key();
    let static_public = x25519::PublicKey::from(&static_private);
    let handshake_max_rate = 100u64;
    let rate_limiter = RateLimiter::new(&static_public, handshake_max_rate);

    // Create a test peer for dev
    let peer_static_public = setup::peer_static_public_key();
    let peer_allowed_ips = setup::peer_allowed_ips();
    let peer_index = 0;
    let test_peer = Arc::new(tokio::sync::Mutex::new(RegisteredPeer {
        public_key: peer_static_public,
        allowed_ips: peer_allowed_ips,
    }));

    // Set of registered peers
    let mut registered_peers = HashMap::new();
    let mut registered_peers_by_idx = HashMap::new();

    registered_peers.insert(peer_static_public, Arc::clone(&test_peer));
    registered_peers_by_idx.insert(peer_index, test_peer);

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
                _ = task_client.recv() => {
                    log::trace!("WireGuard UDP listener: received shutdown");
                    break;
                }
                // Handle tunnel closing
                Some(public_key) = active_peers_task_handles.next() => {
                    match public_key {
                        Ok(public_key) => {
                            log::info!("Removing peer: {public_key:?}");
                            active_peers.remove(&public_key);
                            // TODO: remove from peers_by_ip?
                            // TODO: remove from peers_by_addr
                        }
                        Err(err) => {
                            error!("WireGuard UDP listener: error receiving shutdown from peer: {err}");
                        }
                    }
                },
                // Handle incoming packets
                Ok((len, addr)) = udp_socket.recv_from(&mut buf) => {
                    log::trace!("udp: received {} bytes from {}", len, addr);

                    // If this addr has already been encountered, send directly to tunnel
                    // TODO: optimization oppertunity to instead create a connected UDP socket
                    // inside the wg tunnel, where you can recv the data directly.
                    if let Some(peer_tx) = active_peers_by_addr.get(&addr) {
                        log::info!("udp: received {len} bytes from {addr} from known peer");
                        peer_tx.send(Event::Wg(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();
                    }

                    // Verify the incoming packet
                    let verified_packet = match rate_limiter.verify_packet(Some(addr.ip()), &buf[..len], &mut dst_buf) {
                        Ok(packet) => packet,
                        Err(TunnResult::WriteToNetwork(cookie)) => {
                            log::info!("WireGuard UDP listener: send back cookie");
                            udp_socket.send_to(cookie, addr).await.unwrap();
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
                                    log::warn!("Handshake failed");
                                    continue;
                                };
                                registered_peers.get(&x25519::PublicKey::from(handshake.peer_static_public))
                            },
                            noise::Packet::HandshakeResponse(packet) => {
                                let peer_idx = packet.receiver_idx >> 8;
                                registered_peers_by_idx.get(&peer_idx)
                            },
                            noise::Packet::PacketCookieReply(packet) => {
                                let peer_idx = packet.receiver_idx >> 8;
                                registered_peers_by_idx.get(&peer_idx)
                            },
                            noise::Packet::PacketData(packet) => {
                                let peer_idx = packet.receiver_idx >> 8;
                                registered_peers_by_idx.get(&peer_idx)
                            },
                        };

                        if let Some(reg_peer) = reg_peer {
                            reg_peer.lock().await
                        } else {
                            log::warn!("Peer not registered");
                            continue;
                        }
                    };

                    // Look up if the peer is already connected
                    if let Some(peer_tx) = active_peers.get_mut(&registered_peer.public_key) {
                        // If it is, send it the packet to deal with
                        log::info!("udp: received {len} bytes from {addr} from known peer");
                        peer_tx.send(Event::WgVerified(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();
                    } else {
                        // If it isn't, start a new tunnel
                        log::info!("udp: received {len} bytes from {addr} from unknown peer, starting tunnel");
                        // NOTE: we are not passing in the existing rate_limiter
                        log::warn!("Creating new rate limiter, consider re-using");
                        let (join_handle, peer_tx) = crate::wg_tunnel::start_wg_tunnel(
                            addr,
                            udp_socket.clone(),
                            static_private.clone(),
                            registered_peer.public_key,
                            registered_peer.allowed_ips,
                            peer_index,
                            tun_task_tx.clone(),
                        );

                        peers_by_ip.lock().unwrap().insert(registered_peer.allowed_ips, peer_tx.clone());
                        active_peers_by_addr.insert(addr, peer_tx.clone());

                        peer_tx.send(Event::Wg(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();

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
