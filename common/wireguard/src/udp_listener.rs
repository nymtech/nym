use std::{net::SocketAddr, sync::Arc};

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
    setup::{self, WG_ADDRESS, WG_PORT},
    TunTaskTx,
};

const MAX_PACKET: usize = 65535;

pub(crate) type ActivePeers = DashMap<SocketAddr, mpsc::UnboundedSender<Event>>;
pub(crate) type PeersByIp = NetworkTable<mpsc::UnboundedSender<Event>>;

pub(crate) async fn start_udp_listener(
    tun_task_tx: TunTaskTx,
    active_peers: Arc<ActivePeers>,
    peers_by_ip: Arc<std::sync::Mutex<PeersByIp>>,
    mut task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let wg_address = SocketAddr::new(WG_ADDRESS.parse().unwrap(), WG_PORT);
    log::info!("Starting wireguard UDP listener on {wg_address}");
    let udp_socket = Arc::new(UdpSocket::bind(wg_address).await?);

    // Setup some static keys for development
    let static_private = setup::server_static_private_key();
    let peer_static_public = setup::peer_static_public_key();
    let peer_allowed_ips = setup::peer_allowed_ips();

    tokio::spawn(async move {
        // Each tunnel is run in its own task, and the task handle is stored here so we can remove
        // it from `active_peers` when the tunnel is closed
        let mut active_peers_task_handles = futures::stream::FuturesUnordered::new();
        let mut buf = [0u8; MAX_PACKET];

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::trace!("WireGuard UDP listener: received shutdown");
                    break;
                }
                // Handle tunnel closing
                Some(addr) = active_peers_task_handles.next() => {
                    match addr {
                        Ok(addr) => {
                            log::info!("Removing peer: {addr:?}");
                            active_peers.remove(&addr);
                            // TODO: remove from peers_by_ip
                        }
                        Err(err) => {
                            error!("WireGuard UDP listener: error receiving shutdown from peer: {err}");
                        }
                    }
                },
                // Handle incoming packets
                Ok((len, addr)) = udp_socket.recv_from(&mut buf) => {
                    log::trace!("udp: received {} bytes from {}", len, addr);

                    if let Some(peer_tx) = active_peers.get_mut(&addr) {
                        log::info!("udp: received {len} bytes from {addr} from known peer");
                        peer_tx.send(Event::WgPacket(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();
                    } else {
                        log::info!("udp: received {len} bytes from {addr} from unknown peer, starting tunnel");
                        // TODO: this is a temporary solution for development since this
                        // assumes we know the peer_static_public this corresponds to.
                        // TODO: rework this before production! This is likely not secure!
                        log::warn!("Assuming peer_static_public is known");
                        log::warn!("SECURITY: Rework me to do proper handshake before creating the tunnel!");
                        let (join_handle, peer_tx) = crate::wg_tunnel::start_wg_tunnel(
                            addr,
                            udp_socket.clone(),
                            static_private.clone(),
                            peer_static_public,
                            peer_allowed_ips,
                            tun_task_tx.clone(),
                        );

                        peers_by_ip.lock().unwrap().insert(peer_allowed_ips, peer_tx.clone());

                        peer_tx.send(Event::WgPacket(buf[..len].to_vec().into()))
                            .tap_err(|err| log::error!("{err}"))
                            .unwrap();

                        // WIP(JON): active peers should probably be keyed by peer_static_public
                        // instead. Does this current setup lead to any issues?
                        log::info!("Adding peer: {addr}");
                        active_peers.insert(addr, peer_tx);
                        active_peers_task_handles.push(join_handle);
                    }
                },
            }
        }
        log::info!("WireGuard listener: shutting down");
    });

    Ok(())
}
