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
    tun_task_channel::{
        tun_task_channel, tun_task_response_channel, TunTaskPayload, TunTaskResponseRx,
        TunTaskResponseTx, TunTaskRx, TunTaskTx,
    },
    udp_listener::PeersByIp,
};

fn setup_tokio_tun_device(name: &str, address: Ipv4Addr, netmask: Ipv4Addr) -> tokio_tun::Tun {
    log::info!("Creating TUN device with: address={address}, netmask={netmask}");
    // Read MTU size from env variable NYM_MTU_SIZE, else default to 1420.
    let mtu = std::env::var("NYM_MTU_SIZE")
        .map(|mtu| mtu.parse().expect("NYM_MTU_SIZE must be a valid integer"))
        .unwrap_or(1420);
    log::info!("Using MTU size: {mtu}");
    tokio_tun::Tun::builder()
        .name(name)
        .tap(false)
        .packet_info(false)
        .mtu(mtu)
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

    routing_mode: RoutingMode,
}

pub enum RoutingMode {
    // The routing table, as how wireguard does it
    AllowedIps(AllowedIpsInner),

    // This is an alternative to the routing table, where we just match outgoing source IP with
    // incoming destination IP.
    Nat(NatInner),
}

impl RoutingMode {
    pub fn new_nat() -> Self {
        RoutingMode::Nat(NatInner {
            nat_table: HashMap::new(),
        })
    }

    pub fn new_allowed_ips(peers_by_ip: Arc<tokio::sync::Mutex<PeersByIp>>) -> Self {
        RoutingMode::AllowedIps(AllowedIpsInner { peers_by_ip })
    }
}

pub struct AllowedIpsInner {
    peers_by_ip: Arc<tokio::sync::Mutex<PeersByIp>>,
}

pub struct NatInner {
    nat_table: HashMap<IpAddr, u64>,
}

pub struct TunDeviceConfig {
    pub base_name: String,
    pub ip: Ipv4Addr,
    pub netmask: Ipv4Addr,
}

impl TunDevice {
    pub fn new(
        routing_mode: RoutingMode,
        config: TunDeviceConfig,
    ) -> (Self, TunTaskTx, TunTaskResponseRx) {
        let TunDeviceConfig {
            base_name,
            ip,
            netmask,
        } = config;
        let name = format!("{base_name}%d");

        let tun = setup_tokio_tun_device(&name, ip, netmask);
        log::info!("Created TUN device: {}", tun.name());

        // Channels to communicate with the other tasks
        let (tun_task_tx, tun_task_rx) = tun_task_channel();
        let (tun_task_response_tx, tun_task_response_rx) = tun_task_response_channel();

        let tun_device = TunDevice {
            tun_task_rx,
            tun_task_response_tx,
            tun,
            routing_mode,
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
        if let RoutingMode::Nat(nat_table) = &mut self.routing_mode {
            nat_table.nat_table.insert(src_addr, tag);
        }

        tokio::time::timeout(
            std::time::Duration::from_millis(1000),
            self.tun.write_all(&packet),
        )
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

        match self.routing_mode {
            // This is how wireguard does it, by consulting the AllowedIPs table.
            RoutingMode::AllowedIps(ref peers_by_ip) => {
                let Ok(peers) = tokio::time::timeout(
                    std::time::Duration::from_millis(1000),
                    peers_by_ip.peers_by_ip.as_ref().lock(),
                )
                .await
                else {
                    log::error!("Failed to lock peer");
                    return;
                };

                if let Some(peer_tx) = peers.longest_match(dst_addr).map(|(_, tx)| tx) {
                    log::info!("Forward packet to wg tunnel");
                    tokio::time::timeout(
                        std::time::Duration::from_millis(1000),
                        peer_tx.send(Event::Ip(packet.to_vec().into())),
                    )
                    .await
                    .tap_err(|err| log::error!("Failed to forward packet to wg tunnel: {err}"))
                    .ok();
                    return;
                }
            }

            // But we can also do it by consulting the NAT table.
            RoutingMode::Nat(ref nat_table) => {
                if let Some(tag) = nat_table.nat_table.get(&dst_addr) {
                    log::info!("Forward packet with tag: {tag}");
                    tokio::time::timeout(
                        std::time::Duration::from_millis(1000),
                        self.tun_task_response_tx.send((*tag, packet.to_vec())),
                    )
                    .await
                    .tap_err(|err| log::error!("Failed to foward packet with tag: {err}"))
                    .ok();
                    return;
                }
            }
        }

        log::info!("No peer found, packet dropped");
    }

    pub async fn run(mut self) {
        let mut buf = [0u8; 65535];

        loop {
            tokio::select! {
                // Reading from the TUN device
                len = self.tun.read(&mut buf) => match len {
                    Ok(len) => {
                        let packet = &buf[..len];
                        tokio::time::timeout(
                            std::time::Duration::from_millis(1000),
                            self.handle_tun_read(packet)
                        )
                        .await
                        .tap_err(|_err| log::error!("Failed: handle_tun_read timeout"))
                        .ok();
                    },
                    Err(err) => {
                        log::info!("iface: read error: {err}");
                        // break;
                    }
                },
                // Writing to the TUN device
                Some(data) = self.tun_task_rx.recv() => {
                    tokio::time::timeout(
                        std::time::Duration::from_millis(1000),
                        self.handle_tun_write(data)
                    )
                    .await
                    .tap_err(|_err| log::error!("Failed: handle_tun_write timeout"))
                    .ok();
                }
            }
        }
        // log::info!("TUN device shutting down");
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
