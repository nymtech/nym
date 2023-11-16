use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use etherparse::{InternetSlice, SlicedPacket};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::{
    active_peers::PeerEventSenderError,
    event::Event,
    tun_task_channel::{
        tun_task_channel, tun_task_response_channel, TunTaskPayload, TunTaskResponseRx,
        TunTaskResponseSendError, TunTaskResponseTx, TunTaskRx, TunTaskTx,
    },
    udp_listener::PeersByIp,
};

const MUTEX_LOCK_TIMEOUT_MS: u64 = 200;
const TUN_WRITE_TIMEOUT_MS: u64 = 1000;

#[derive(thiserror::Error, Debug)]
pub enum TunDeviceError {
    #[error("timeout writing to tun device, dropping packet")]
    TunWriteTimeout,

    #[error("failed forwarding packet to peer: {source}")]
    ForwardToPeerFailed {
        #[from]
        source: PeerEventSenderError,
    },

    #[error("failed to forward responding packet with tag: {source}")]
    ForwardNatResponseFailed {
        #[from]
        source: TunTaskResponseSendError,
    },

    #[error("error writing to tun device: {source}")]
    TunWriteError { source: std::io::Error },

    #[error("unable to parse destination address from packet")]
    UnableToParseDstAdddress,

    #[error("unable to parse source address from packet")]
    UnableToParseSrcAddress {
        #[from]
        source: etherparse::ReadError,
    },

    #[error("unable to parse source address from packet: ip header missing")]
    UnableToParseSrcAddressIpHeaderMissing,

    #[error("unable to lock peer mutex")]
    FailedToLockPeer,
}

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

impl AllowedIpsInner {
    async fn lock(&self) -> Result<tokio::sync::MutexGuard<PeersByIp>, TunDeviceError> {
        timeout(
            Duration::from_millis(MUTEX_LOCK_TIMEOUT_MS),
            self.peers_by_ip.as_ref().lock(),
        )
        .await
        .map_err(|_| TunDeviceError::FailedToLockPeer)
    }
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
    async fn handle_tun_write(&mut self, data: TunTaskPayload) -> Result<(), TunDeviceError> {
        let (tag, packet) = data;
        let dst_addr = boringtun::noise::Tunn::dst_address(&packet)
            .ok_or_else(|| TunDeviceError::UnableToParseDstAdddress)?;

        let src_addr = parse_src_address(&packet)?;
        log::info!(
            "iface: write Packet({src_addr} -> {dst_addr}, {} bytes)",
            packet.len()
        );

        // TODO: expire old entries
        if let RoutingMode::Nat(nat_table) = &mut self.routing_mode {
            nat_table.nat_table.insert(src_addr, tag);
        }

        timeout(
            Duration::from_millis(TUN_WRITE_TIMEOUT_MS),
            self.tun.write_all(&packet),
        )
        .await
        .map_err(|_| TunDeviceError::TunWriteTimeout)?
        .map_err(|err| TunDeviceError::TunWriteError { source: err })
    }

    // Receive reponse packets from the wild internet
    async fn handle_tun_read(&self, packet: &[u8]) -> Result<(), TunDeviceError> {
        let dst_addr = boringtun::noise::Tunn::dst_address(packet)
            .ok_or(TunDeviceError::UnableToParseDstAdddress)?;
        let src_addr = parse_src_address(packet)?;
        log::info!(
            "iface: read Packet({src_addr} -> {dst_addr}, {} bytes)",
            packet.len(),
        );

        // Route packet to the correct peer.

        match self.routing_mode {
            // This is how wireguard does it, by consulting the AllowedIPs table.
            RoutingMode::AllowedIps(ref peers_by_ip) => {
                let peers = peers_by_ip.lock().await?;
                if let Some(peer_tx) = peers.longest_match(dst_addr).map(|(_, tx)| tx) {
                    log::info!("Forward packet to wg tunnel");
                    return peer_tx
                        .send(Event::Ip(packet.to_vec().into()))
                        .await
                        .map_err(|err| err.into());
                }
            }

            // But we can also do it by consulting the NAT table.
            RoutingMode::Nat(ref nat_table) => {
                if let Some(tag) = nat_table.nat_table.get(&dst_addr) {
                    log::info!("Forward packet with NAT tag: {tag}");
                    return self
                        .tun_task_response_tx
                        .send((*tag, packet.to_vec()))
                        .await
                        .map_err(|err| err.into());
                }
            }
        }

        log::info!("No peer found, packet dropped");
        Ok(())
    }

    pub async fn run(mut self) {
        let mut buf = [0u8; 65535];

        loop {
            tokio::select! {
                // Reading from the TUN device
                len = self.tun.read(&mut buf) => match len {
                    Ok(len) => {
                        let packet = &buf[..len];
                        if let Err(err) = self.handle_tun_read(packet).await {
                            log::error!("iface: handle_tun_read failed: {err}")
                        }
                    },
                    Err(err) => {
                        log::info!("iface: read error: {err}");
                        // break;
                    }
                },
                // Writing to the TUN device
                Some(data) = self.tun_task_rx.recv() => {
                    if let Err(err) = self.handle_tun_write(data).await {
                        log::error!("ifcae: handle_tun_write failed: {err}");
                    }
                }
            }
        }
        // log::info!("TUN device shutting down");
    }

    pub fn start(self) {
        tokio::spawn(async move { self.run().await });
    }
}

fn parse_src_address(packet: &[u8]) -> Result<IpAddr, TunDeviceError> {
    let headers = SlicedPacket::from_ip(packet)?;
    match headers.ip {
        Some(InternetSlice::Ipv4(ip, _)) => Ok(ip.source_addr().into()),
        Some(InternetSlice::Ipv6(ip, _)) => Ok(ip.source_addr().into()),
        None => Err(TunDeviceError::UnableToParseSrcAddressIpHeaderMissing),
    }
}
