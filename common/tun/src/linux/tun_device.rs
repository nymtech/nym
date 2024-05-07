use std::net::Ipv6Addr;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use etherparse::{InternetSlice, SlicedPacket};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::tun_task_channel::{
    tun_task_channel, tun_task_response_channel, TunTaskPayload, TunTaskResponseRx,
    TunTaskResponseSendError, TunTaskResponseTx, TunTaskRx, TunTaskTx,
};

const TUN_WRITE_TIMEOUT_MS: u64 = 1000;

#[derive(thiserror::Error, Debug)]
pub enum TunDeviceError {
    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("{0}")]
    TokioTun(#[from] tokio_tun::Error),

    #[error("timeout writing to tun device, dropping packet")]
    TunWriteTimeout,

    #[error("error writing to tun device: {source}")]
    TunWriteError { source: std::io::Error },

    #[error("failed to forward responding packet with tag: {source}")]
    ForwardNatResponseFailed {
        #[from]
        source: TunTaskResponseSendError,
    },

    #[error("unable to parse headers in packet")]
    UnableToParseHeaders {
        #[from]
        source: etherparse::ReadError,
    },

    #[error("unable to parse src and dst address from packet: ip header missing")]
    UnableToParseAddressIpHeaderMissing,

    #[error("unable to lock peer mutex")]
    FailedToLockPeer,
}

fn setup_tokio_tun_device(
    name: &str,
    address: Ipv4Addr,
    netmask: Ipv4Addr,
) -> Result<tokio_tun::Tun, TunDeviceError> {
    log::info!("Creating TUN device with: address={address}, netmask={netmask}");
    // Read MTU size from env variable NYM_MTU_SIZE, else default to 1420.
    let mtu = std::env::var("NYM_MTU_SIZE")
        .map(|mtu| mtu.parse().expect("NYM_MTU_SIZE must be a valid integer"))
        .unwrap_or(1420);
    log::info!("Using MTU size: {mtu}");
    Ok(tokio_tun::Tun::builder()
        .name(name)
        .tap(false)
        .packet_info(false)
        .mtu(mtu)
        .up()
        .address(address)
        .netmask(netmask)
        .try_build()?)
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
    // This is an alternative to the routing table, where we just match outgoing source IP with
    // incoming destination IP.
    Nat(NatInner),

    // Just forward without checking anything
    Passthrough,
}

impl RoutingMode {
    pub fn new_nat() -> Self {
        RoutingMode::Nat(NatInner {
            nat_table: HashMap::new(),
        })
    }

    pub fn new_passthrough() -> Self {
        RoutingMode::Passthrough
    }
}

pub struct NatInner {
    nat_table: HashMap<IpAddr, u64>,
}

pub struct TunDeviceConfig {
    pub base_name: String,
    pub ipv4: Ipv4Addr,
    pub netmaskv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
    pub netmaskv6: String,
}

impl TunDevice {
    pub fn new(
        routing_mode: RoutingMode,
        config: TunDeviceConfig,
    ) -> Result<(Self, TunTaskTx, TunTaskResponseRx), TunDeviceError> {
        let tun = Self::new_device_only(config)?;

        // Channels to communicate with the other tasks
        let (tun_task_tx, tun_task_rx) = tun_task_channel();
        let (tun_task_response_tx, tun_task_response_rx) = tun_task_response_channel();

        let tun_device = TunDevice {
            tun_task_rx,
            tun_task_response_tx,
            tun,
            routing_mode,
        };

        Ok((tun_device, tun_task_tx, tun_task_response_rx))
    }

    pub fn new_device_only(config: TunDeviceConfig) -> Result<tokio_tun::Tun, TunDeviceError> {
        let TunDeviceConfig {
            base_name,
            ipv4,
            netmaskv4,
            ipv6,
            netmaskv6,
        } = config;
        let name = format!("{base_name}%d");

        let tun = setup_tokio_tun_device(&name, ipv4, netmaskv4)?;
        log::info!("Created TUN device: {}", tun.name());
        std::process::Command::new("ip")
            .args([
                "-6",
                "addr",
                "add",
                &format!("{}/{}", ipv6, netmaskv6),
                "dev",
                (tun.name()),
            ])
            .output()?;
        Ok(tun)
    }

    // Send outbound packets out on the wild internet
    async fn handle_tun_write(&mut self, data: TunTaskPayload) -> Result<(), TunDeviceError> {
        let (tag, packet) = data;
        let ParsedAddresses { src_addr, dst_addr } = parse_src_dst_address(&packet)?;
        log::debug!(
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
        let ParsedAddresses { src_addr, dst_addr } = parse_src_dst_address(packet)?;
        log::debug!(
            "iface: read Packet({dst_addr} <- {src_addr}, {} bytes)",
            packet.len(),
        );

        // Route packet to the correct peer.

        match self.routing_mode {
            // But we can also do it by consulting the NAT table.
            RoutingMode::Nat(ref nat_table) => {
                if let Some(tag) = nat_table.nat_table.get(&dst_addr) {
                    log::debug!("Forward packet with NAT tag: {tag}");
                    return self
                        .tun_task_response_tx
                        .try_send((*tag, packet.to_vec()))
                        .map_err(|err| err.into());
                }
            }

            RoutingMode::Passthrough => {
                // TODO: skip the parsing at the top of the function
                log::debug!("Forward packet without checking anything");
                return self
                    .tun_task_response_tx
                    .try_send((0, packet.to_vec()))
                    .map_err(|err| err.into());
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
                        log::error!("iface: handle_tun_write failed: {err}");
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

struct ParsedAddresses {
    src_addr: IpAddr,
    dst_addr: IpAddr,
}

fn parse_src_dst_address(packet: &[u8]) -> Result<ParsedAddresses, TunDeviceError> {
    let headers = SlicedPacket::from_ip(packet)?;
    match headers.ip {
        Some(InternetSlice::Ipv4(ip, _)) => Ok(ParsedAddresses {
            src_addr: ip.source_addr().into(),
            dst_addr: ip.destination_addr().into(),
        }),
        Some(InternetSlice::Ipv6(ip, _)) => Ok(ParsedAddresses {
            src_addr: ip.source_addr().into(),
            dst_addr: ip.destination_addr().into(),
        }),
        None => Err(TunDeviceError::UnableToParseAddressIpHeaderMissing),
    }
}
