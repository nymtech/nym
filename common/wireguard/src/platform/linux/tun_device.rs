use std::{net::Ipv4Addr, sync::Arc};

use etherparse::{InternetSlice, SlicedPacket};
use tap::TapFallible;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{self},
};

use crate::{
    event::Event,
    setup::{TUN_BASE_NAME, TUN_DEVICE_ADDRESS, TUN_DEVICE_NETMASK},
    udp_listener::PeersByIp,
    TunTaskTx,
};

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

pub struct TunDevice {
    // The TUN device that we read/write to, to send/receive packets
    tun: tokio_tun::Tun,

    // Incoming data that we should send
    tun_task_rx: mpsc::UnboundedReceiver<Vec<u8>>,

    // The routing table.
    // An alternative would be to do NAT by just matching incoming with outgoing.
    peers_by_ip: Arc<std::sync::Mutex<PeersByIp>>,
}

impl TunDevice {
    pub fn new(peers_by_ip: Arc<std::sync::Mutex<PeersByIp>>) -> (Self, TunTaskTx) {
        let tun = setup_tokio_tun_device(
            format!("{TUN_BASE_NAME}%d").as_str(),
            TUN_DEVICE_ADDRESS.parse().unwrap(),
            TUN_DEVICE_NETMASK.parse().unwrap(),
        );
        log::info!("Created TUN device: {}", tun.name());

        // Channels to communicate with the other tasks
        let (tun_task_tx, tun_task_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let tun_task_tx = TunTaskTx(tun_task_tx);

        let tun_device = TunDevice {
            tun_task_rx,
            tun,
            peers_by_ip,
        };

        (tun_device, tun_task_tx)
    }

    pub async fn run(mut self) {
        let mut buf = [0u8; 1024];

        loop {
            tokio::select! {
                // Reading from the TUN device
                len = self.tun.read(&mut buf) => match len {
                    Ok(len) => {
                        let packet = &buf[..len];
                        let dst_addr = boringtun::noise::Tunn::dst_address(packet).unwrap();

                        let headers = SlicedPacket::from_ip(packet).unwrap();
                        let src_addr = match headers.ip.unwrap() {
                            InternetSlice::Ipv4(ip, _) => ip.source_addr().to_string(),
                            InternetSlice::Ipv6(ip, _) => ip.source_addr().to_string(),
                        };
                        log::info!("iface: read Packet({src_addr} -> {dst_addr}, {len} bytes)");

                        // Route packet to the correct peer.
                        if let Some(peer_tx) = self.peers_by_ip.lock().unwrap().longest_match(dst_addr).map(|(_, tx)| tx) {
                            log::info!("Forward packet to wg tunnel");
                            peer_tx
                                .send(Event::Ip(packet.to_vec().into()))
                                .tap_err(|err| log::error!("{err}"))
                                .unwrap();
                        } else {
                            log::info!("No peer found, packet dropped");
                        }
                    },
                    Err(err) => {
                        log::info!("iface: read error: {err}");
                        break;
                    }
                },

                // Writing to the TUN device
                Some(data) = self.tun_task_rx.recv() => {
                    let headers = SlicedPacket::from_ip(&data).unwrap();
                    let (source_addr, destination_addr) = match headers.ip.unwrap() {
                        InternetSlice::Ipv4(ip, _) => (ip.source_addr(), ip.destination_addr()),
                        InternetSlice::Ipv6(_, _) => unimplemented!(),
                    };

                    log::info!(
                        "iface: write Packet({source_addr} -> {destination_addr}, {} bytes)",
                        data.len()
                    );
                    self.tun.write_all(&data).await.unwrap();
                }
            }
        }
        log::info!("TUN device shutting down");
    }

    pub fn start(self) {
        tokio::spawn(async move { self.run().await });
    }
}
