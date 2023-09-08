use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use base64::{engine::general_purpose, Engine as _};
use boringtun::x25519;
use futures::StreamExt;
use log::{error, info};
use nym_task::TaskClient;
use tap::TapFallible;
use tokio::{net::UdpSocket, sync::mpsc, task::JoinHandle};
use tun::WireGuardTunnel;

use crate::event::Event;

mod event;
mod tun;

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
        let mut buf = [0u8; MAX_PACKET];

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
