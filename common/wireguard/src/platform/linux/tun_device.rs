use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use etherparse::{InternetSlice, SlicedPacket};
use tap::TapFallible;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    event::Event,
    setup::{TUN_BASE_NAME, TUN_DEVICE_ADDRESS, TUN_DEVICE_NETMASK},
    tun_task_channel::{
        tun_task_channel, tun_task_response_channel, TunTaskPayload, TunTaskResponseRx,
        TunTaskResponseTx, TunTaskRx, TunTaskTx,
    },
    udp_listener::PeersByIp,
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
    tun_task_rx: TunTaskRx,

    // And when we get replies, this is where we should send it
    tun_task_response_tx: TunTaskResponseTx,

    // The routing table, as how wireguard does it
    peers_by_ip: Arc<tokio::sync::Mutex<PeersByIp>>,

    // This is an alternative to the routing table, where we just match outgoing source IP with
    // incoming destination IP.
    nat_table: HashMap<IpAddr, u64>,
}

impl TunDevice {
    pub fn new(
        peers_by_ip: Arc<tokio::sync::Mutex<PeersByIp>>,
    ) -> (Self, TunTaskTx, TunTaskResponseRx) {
        let tun = setup_tokio_tun_device(
            format!("{TUN_BASE_NAME}%d").as_str(),
            TUN_DEVICE_ADDRESS.parse().unwrap(),
            TUN_DEVICE_NETMASK.parse().unwrap(),
        );
        log::info!("Created TUN device: {}", tun.name());

        // Channels to communicate with the other tasks
        let (tun_task_tx, tun_task_rx) = tun_task_channel();
        let (tun_task_response_tx, tun_task_response_rx) = tun_task_response_channel();

        let tun_device = TunDevice {
            tun_task_rx,
            tun_task_response_tx,
            tun,
            peers_by_ip,
            nat_table: HashMap::new(),
        };

        (tun_device, tun_task_tx, tun_task_response_rx)
    }

    // Send outbound packets out on the wild internet
    async fn handle_tun_write(&mut self, data: TunTaskPayload) {
        let (tag, packet) = data;
        let Some(dst_addr) = boringtun::noise::Tunn::dst_address(&packet) else {
            log::error!("Unable to parse dst_address in packet that was supposed to be written to tun device");
            return;
        };
        let Some(src_addr) = parse_src_address(&packet) else {
            log::error!("Unable to parse src_address in packet that was supposed to be written to tun device");
            return;
        };
        log::info!(
            "iface: write Packet({src_addr} -> {dst_addr}, {} bytes)",
            packet.len()
        );

        // TODO: expire old entries
        self.nat_table.insert(src_addr, tag);

        self.tun
            .write_all(&packet)
            .await
            .tap_err(|err| {
                log::error!("iface: write error: {err}");
            })
            .ok();
    }

    // Receive reponse packets from the wild internet
    async fn handle_tun_read(&self, packet: &[u8]) {
        let Some(dst_addr) = boringtun::noise::Tunn::dst_address(packet) else {
            log::error!("Unable to parse dst_address in packet that was read from tun device");
            return;
        };
        let Some(src_addr) = parse_src_address(packet) else {
            log::error!("Unable to parse src_address in packet that was read from tun device");
            return;
        };
        log::info!(
            "iface: read Packet({src_addr} -> {dst_addr}, {} bytes)",
            packet.len(),
        );

        // Route packet to the correct peer.

        // This is how wireguard does it, by consulting the AllowedIPs table.
        if false {
            let peers = self.peers_by_ip.lock().await;
            if let Some(peer_tx) = peers.longest_match(dst_addr).map(|(_, tx)| tx) {
                log::info!("Forward packet to wg tunnel");
                peer_tx
                    .send(Event::Ip(packet.to_vec().into()))
                    .await
                    .tap_err(|err| log::error!("{err}"))
                    .ok();
                return;
            }
        }

        // But we do it by consulting the NAT table.
        {
            if let Some(tag) = self.nat_table.get(&dst_addr) {
                log::info!("Forward packet to wg tunnel with tag: {tag}");
                self.tun_task_response_tx
                    .send((*tag, packet.to_vec()))
                    .await
                    .tap_err(|err| log::error!("{err}"))
                    .ok();
                return;
            }
        }

        log::info!("No peer found, packet dropped");
    }

    pub async fn run(mut self) {
        let mut buf = [0u8; 1024];

        loop {
            tokio::select! {
                // Reading from the TUN device
                len = self.tun.read(&mut buf) => match len {
                    Ok(len) => {
                        let packet = &buf[..len];
                        self.handle_tun_read(packet).await;
                    },
                    Err(err) => {
                        log::info!("iface: read error: {err}");
                        break;
                    }
                },
                // Writing to the TUN device
                Some(data) = self.tun_task_rx.recv() => {
                    self.handle_tun_write(data).await;
                }
            }
        }
        log::info!("TUN device shutting down");
    }

    pub fn start(self) {
        tokio::spawn(async move { self.run().await });
    }
}

fn parse_src_address(packet: &[u8]) -> Option<IpAddr> {
    let headers = SlicedPacket::from_ip(packet)
        .tap_err(|err| log::error!("Unable to parse IP packet: {err:?}"))
        .ok()?;
    Some(match headers.ip? {
        InternetSlice::Ipv4(ip, _) => ip.source_addr().into(),
        InternetSlice::Ipv6(ip, _) => ip.source_addr().into(),
    })
}
