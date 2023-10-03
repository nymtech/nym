use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use base64::{engine::general_purpose, Engine as _};
use boringtun::x25519;
use dashmap::DashMap;
use etherparse::{InternetSlice, SlicedPacket};
use futures::StreamExt;
use log::{error, info};
use nym_task::TaskClient;
use tap::TapFallible;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UdpSocket,
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
};
use tun::WireGuardTunnel;

use crate::event::Event;

pub use error::WgError;

mod error;
mod event;
mod tun;

// The wireguard UDP listener
const WG_ADDRESS: &str = "0.0.0.0";
const WG_PORT: u16 = 51822;

// The interface used to route traffic
const TUN_BASE_NAME: &str = "nymtun";
const TUN_DEVICE_ADDRESS: &str = "10.0.0.1";
const TUN_DEVICE_NETMASK: &str = "255.255.255.0";

// The private key of the listener
// Corresponding public key: "WM8s8bYegwMa0TJ+xIwhk+dImk2IpDUKslDBCZPizlE="
const PRIVATE_KEY: &str = "AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=";

// The public keys of the registered peers (clients)
const PEERS: &[&str; 1] = &[
    // Corresponding private key: "ILeN6gEh6vJ3Ju8RJ3HVswz+sPgkcKtAYTqzQRhTtlo="
    "NCIhkgiqxFx1ckKl3Zuh595DzIFl8mxju1Vg995EZhI=",
    // Another key
    // "mxV/mw7WZTe+0Msa0kvJHMHERDA/cSskiZWQce+TdEs=",
];

const MAX_PACKET: usize = 65535;

type ActivePeers = DashMap<SocketAddr, mpsc::UnboundedSender<Event>>;

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
    endpoint: SocketAddr,
    udp: Arc<UdpSocket>,
    static_private: x25519::StaticSecret,
    peer_static_public: x25519::PublicKey,
    tunnel_tx: UnboundedSender<Vec<u8>>,
) -> (JoinHandle<SocketAddr>, mpsc::UnboundedSender<Event>) {
    let (mut tunnel, peer_tx) =
        WireGuardTunnel::new(udp, endpoint, static_private, peer_static_public, tunnel_tx);
    let join_handle = tokio::spawn(async move {
        tunnel.spin_off().await;
        endpoint
    });
    (join_handle, peer_tx)
}

fn setup_tokio_tun_device(name: &str, address: Ipv4Addr, netmask: Ipv4Addr) -> tokio_tun::Tun {
    log::info!("Creating TUN device with: address={address}, netmask={netmask}");
    tokio_tun::Tun::builder()
        .name(name)
        .tap(false)
        .packet_info(false)
        .mtu(1350)
        .up()
        .address(address)
        .netmask(netmask)
        .try_build()
        .expect("Failed to setup tun device, do you have permission?")
}

fn start_tun_device(_active_peers: Arc<ActivePeers>) -> UnboundedSender<Vec<u8>> {
    let tun = setup_tokio_tun_device(
        format!("{}%d", TUN_BASE_NAME).as_str(),
        TUN_DEVICE_ADDRESS.parse().unwrap(),
        TUN_DEVICE_NETMASK.parse().unwrap(),
    );
    log::info!("Created TUN device: {}", tun.name());

    let (mut tun_device_rx, mut tun_device_tx) = tokio::io::split(tun);

    // Channels to communicate with the other tasks
    let (tun_task_tx, mut tun_task_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        loop {
            tokio::select! {
                // Reading from the TUN device
                len = tun_device_rx.read(&mut buf) => match len {
                    Ok(len) => {
                        let packet = &buf[..len];
                        let dst_addr = boringtun::noise::Tunn::dst_address(packet).unwrap();

                        let headers = SlicedPacket::from_ip(packet).unwrap();
                        let src_addr = match headers.ip.unwrap() {
                            InternetSlice::Ipv4(ip, _) => ip.source_addr().to_string(),
                            InternetSlice::Ipv6(ip, _) => ip.source_addr().to_string(),
                        };
                        log::info!("iface: read Packet({src_addr} -> {dst_addr}, {len} bytes)");

                        // TODO: route packet to the correct peer.
                        log::info!("...forward packet to the correct peer (NOT YET IMPLEMENTED)");
                    },
                    Err(err) => {
                        log::info!("iface: read error: {err}");
                        break;
                    }
                },

                // Writing to the TUN device
                Some(data) = tun_task_rx.recv() => {
                    let headers = SlicedPacket::from_ip(&data).unwrap();
                    let (source_addr, destination_addr) = match headers.ip.unwrap() {
                        InternetSlice::Ipv4(ip, _) => (ip.source_addr(), ip.destination_addr()),
                        InternetSlice::Ipv6(_, _) => unimplemented!(),
                    };

                    log::info!(
                        "iface: write Packet({source_addr} -> {destination_addr}, {} bytes)",
                        data.len()
                    );
                    // log::info!("iface: writing {} bytes", data.len());
                    tun_device_tx.write_all(&data).await.unwrap();
                }
            }
        }
        log::info!("TUN device shutting down");
    });
    tun_task_tx
}

async fn start_udp_listener(
    tun_task_tx: UnboundedSender<Vec<u8>>,
    active_peers: Arc<ActivePeers>,
    mut task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let wg_address = SocketAddr::new(WG_ADDRESS.parse().unwrap(), WG_PORT);
    log::info!("Starting wireguard UDP listener on {wg_address}");
    let udp_socket = Arc::new(UdpSocket::bind(wg_address).await?);

    // Setup some static keys for development
    let (static_private, peer_static_public) = init_static_dev_keys();

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
                        let (join_handle, peer_tx) = start_wg_tunnel(
                            addr,
                            udp_socket.clone(),
                            static_private.clone(),
                            peer_static_public,
                            tun_task_tx.clone(),
                        );
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

pub async fn start_wireguard(
    task_client: TaskClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // The set of active tunnels indexed by the peer's address
    let active_peers: Arc<ActivePeers> = Arc::new(ActivePeers::new());

    // Start the tun device that is used to relay traffic outbound
    let tun_task_tx = start_tun_device(active_peers.clone());

    // Start the UDP listener that clients connect to
    start_udp_listener(tun_task_tx, active_peers, task_client).await?;

    Ok(())
}
