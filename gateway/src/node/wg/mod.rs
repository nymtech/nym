use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use async_recursion::async_recursion;
use base64::engine::general_purpose;
use base64::Engine as _;
use boringtun::noise::errors::WireGuardError;
use boringtun::noise::{Tunn, TunnResult};
use etherparse::{InternetSlice, PacketBuilder, SlicedPacket, TransportSlice};
use log::{debug, info, warn};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{MutablePacket, Packet, PacketSize};
use pnet::transport::{ipv4_packet_iter, transport_channel};
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout};
use x25519_dalek::StaticSecret;

use crate::error;

use self::events::Event;

pub mod events;

const MAX_PACKET: usize = 65536;

/// A WireGuard tunnel. Encapsulates and decapsulates IP packets,
/// recieves packets from the client on the udp_rx channel,
/// and events from the internet on the eth_rx channel,
/// sends data through udp socket or datalink sender directly.
/// For now all tunnels recieve all events and filter on the source_peer_addr
pub struct WireGuardTunnel {
    source_peer_addr: Arc<RwLock<Option<(Ipv4Addr, u16)>>>,
    /// `boringtun` peer/tunnel implementation, used for crypto & WG protocol.
    peer: Arc<Mutex<Tunn>>,
    udp: Arc<UdpSocket>,
    peer_endpoint: SocketAddr,
    bus_rx: tokio::sync::broadcast::Receiver<Event>,
    bus_tx: tokio::sync::broadcast::Sender<Event>,
}

pub fn handle_l3_packet(data: &[u8], destination_addr: Ipv4Addr) -> Vec<u8> {
    let (mut tx, mut rx) = transport_channel(
        65535,
        pnet::transport::TransportChannelType::Layer3(IpNextHeaderProtocols::Tcp),
    )
    .unwrap();

    let mut rx_iterator = ipv4_packet_iter(&mut rx);

    let mut must_send = true;
    let mut cnt = 0;
    while let Ok((packet, addr)) = rx_iterator.next() {
        if must_send {
            let data = data.to_vec();
            let incoming_packet = Ipv4Packet::new(&data).unwrap();
            let mut new_packet = vec![0; incoming_packet.packet_size()];
            let mut outgoing_packet = MutableIpv4Packet::new(&mut new_packet).unwrap();
            outgoing_packet.clone_from(&incoming_packet);
            info!(
                "Sending (ttl={}, proto={} from {} to {}({})",
                outgoing_packet.get_ttl(),
                outgoing_packet.get_next_level_protocol(),
                outgoing_packet.get_source(),
                outgoing_packet.get_destination(),
                destination_addr
            );
            outgoing_packet.set_source("95.217.227.118".parse().unwrap());
            let sent = tx
                .send_to(outgoing_packet, IpAddr::V4(destination_addr))
                .unwrap();
            info!("Sent L3 packet ({sent})");
            must_send = false;
            continue;
        }
        cnt += 1;
        let source = packet.get_source();
        let destination = packet.get_destination();
        info!("Ignoring packet from {source}");

        if source == destination_addr {
            info!("({addr}){source} -> {destination}");
            return packet.payload().to_vec();
        }
        if cnt >= 10 {
            break;
        }
    }
    vec![]
}

impl WireGuardTunnel {
    async fn set_source_peer_addr(&self, source_addr: Ipv4Addr, source_port: Option<u16>) {
        {
            if self.source_peer_addr.read().await.is_some() {
                return;
            }
        }
        let mut source_peer_addr = self.source_peer_addr.write().await;
        *source_peer_addr = Some((source_addr, source_port.unwrap_or(0)))
    }

    pub async fn spin_off(mut self) {
        info!("Spun off WG tunnel");
        // We'll receive both inbound and outbound packages on the same channel, and filter on packet type
        loop {
            tokio::select! {
                    packet = self.bus_rx.recv() => {
                        match packet {
                            Ok(p) => {
                                info!("{p}");
                                match p {
                                    Event::IpPacket(data) => self.consume_eth(&data).await,
                                    Event::WgPacket(data) => self.consume_wg(&data).await,
                                    _ => {}
                                }
                            },
                            Err(e) => error!("{e}")
                        }
                    },
                    _ = sleep(Duration::from_millis(5))=> {
                        let mut send_buf = [0u8; MAX_PACKET];
                        let tun_result = {
                            let mut tun = timeout(Duration::from_millis(100), self.peer()).await.unwrap();
                            tun.update_timers(&mut send_buf)
                        };
                        self.handle_routine_tun_result(tun_result).await;
                    }
            }
        }
    }

    pub async fn consume_eth(&self, data: &[u8]) {
        let parsed_packet = SlicedPacket::from_ethernet(data).unwrap();
        debug!("{parsed_packet:?}");
        let (source_addr, destination_addr) = match parsed_packet.ip.unwrap() {
            InternetSlice::Ipv4(ip, _) => (ip.source_addr(), ip.destination_addr()),
            _ => unimplemented!(),
        };
        let (source_port, destination_port, icmp_type) = match parsed_packet.transport.as_ref() {
            Some(TransportSlice::Tcp(tcp)) => {
                (Some(tcp.source_port()), Some(tcp.destination_port()), None)
            }
            Some(TransportSlice::Udp(udp)) => {
                (Some(udp.source_port()), Some(udp.destination_port()), None)
            }
            Some(TransportSlice::Icmpv4(icmp)) => (None, None, Some(icmp.icmp_type())),
            Some(TransportSlice::Icmpv6(_)) => panic!("ICMPv6"),
            Some(TransportSlice::Unknown(_)) => panic!("Unknown"),
            None => panic!("No transport layer"),
        };
        debug!(
            "{:?}:{:?} -> {:?}:{:?} - ({:?})",
            source_addr, source_port, destination_addr, destination_port, icmp_type
        );

        if destination_addr == self.source_peer_addr.read().await.unwrap().0 {
            info!("Sending {} to {}", data.len(), self.peer_endpoint);
        } else {
            return;
        }

        let response_packet_builder =
            PacketBuilder::ipv4(source_addr.octets(), destination_addr.octets(), 64);

        let mut response_packet =
            Vec::<u8>::with_capacity(response_packet_builder.size(parsed_packet.payload.len()));

        match parsed_packet.transport.as_ref() {
            Some(TransportSlice::Udp(udp)) => {
                debug!("UDP: {}, {}", udp.length(), udp.destination_port());
                let response_packet_builder =
                    response_packet_builder.udp(source_port.unwrap(), destination_port.unwrap());
                response_packet_builder
                    .write(&mut response_packet, parsed_packet.payload)
                    .unwrap();
            }
            Some(TransportSlice::Tcp(tcp)) => {
                let response_packet_builder = response_packet_builder.tcp(
                    destination_port.unwrap(),
                    source_port.unwrap(),
                    tcp.sequence_number(),
                    tcp.window_size(),
                );
                response_packet_builder
                    .write(&mut response_packet, parsed_packet.payload)
                    .unwrap();
            }
            Some(TransportSlice::Icmpv4(icmp)) => {
                info!("{:?}", icmp.icmp_type());
                let response_packet_builder = response_packet_builder.icmpv4(icmp.icmp_type());
                response_packet_builder
                    .write(&mut response_packet, parsed_packet.payload)
                    .unwrap();
            }
            None => {}
            _ => unimplemented!(),
        };

        let encapsulated_response_packet = self.encapsulate_packet(&response_packet).await;
        // let packet = Tunn::parse_incoming_packet(&response_packet).unwrap();
        // info!("Sending {packet:?} to {addr}");
        let sent = self
            .udp
            .send_to(&encapsulated_response_packet, self.peer_endpoint)
            .await
            .unwrap();
        info!(
            "[{}:{} ({sent})-> {}:{}] -> {}",
            destination_addr,
            destination_port.unwrap_or(0),
            source_addr,
            source_port.unwrap_or(0),
            self.peer_endpoint
        );
    }

    // TODO: extend to work with IPv6
    pub async fn produce_eth(&self, packet_bytes: &[u8]) -> Vec<u8> {
        let outgoing_packet = SlicedPacket::from_ip(packet_bytes).unwrap();
        let (source_addr, destination_addr) = match outgoing_packet.ip.unwrap() {
            InternetSlice::Ipv4(ip, _) => (ip.source_addr(), ip.destination_addr()),
            _ => unimplemented!(),
        };
        let (source_port, destination_port, icmp_type) = match outgoing_packet.transport.as_ref() {
            Some(TransportSlice::Tcp(tcp)) => {
                (Some(tcp.source_port()), Some(tcp.destination_port()), None)
            }
            Some(TransportSlice::Udp(udp)) => {
                (Some(udp.source_port()), Some(udp.destination_port()), None)
            }
            Some(TransportSlice::Icmpv4(icmp)) => (None, None, Some(icmp.icmp_type())),
            Some(TransportSlice::Icmpv6(_)) => panic!("ICMPv6"),
            Some(TransportSlice::Unknown(_)) => panic!("Unknown"),
            None => panic!("No transport layer"),
        };
        info!(
            "{:?}:{:?} -> {:?}:{:?} - ({:?})",
            source_addr, source_port, destination_addr, destination_port, icmp_type
        );
        self.set_source_peer_addr(source_addr, source_port).await;
        handle_l3_packet(packet_bytes, destination_addr)
    }

    /// WireGuard consumption task. Receives encrypted packets from the WireGuard peer,
    /// decapsulates them, and dispatches newly received IP packets.
    async fn consume_wg(&self, data: &[u8]) {
        let mut send_buf = [0u8; MAX_PACKET];
        let mut peer = self.peer().await;
        match peer.decapsulate(None, data, &mut send_buf) {
            TunnResult::WriteToNetwork(packet) => {
                match self.udp.send_to(packet, self.peer_endpoint).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to send decapsulation-instructed packet to WireGuard endpoint: {:?}", e);
                    }
                };
                loop {
                    let mut send_buf = [0u8; MAX_PACKET];
                    match peer.decapsulate(None, &[], &mut send_buf) {
                        TunnResult::WriteToNetwork(packet) => {
                            match self.udp.send_to(packet, self.peer_endpoint).await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("Failed to send decapsulation-instructed packet to WireGuard endpoint: {:?}", e);
                                    break;
                                }
                            };
                        }
                        _ => {
                            break;
                        }
                    }
                }
            }
            TunnResult::WriteToTunnelV4(packet, _) | TunnResult::WriteToTunnelV6(packet, _) => {
                info!(
                    "WireGuard endpoint sent IP packet of {} bytes",
                    packet.len()
                );
                let response = self.produce_eth(packet).await;
                if !response.is_empty() {
                    self.bus_tx.send(Event::IpPacket(response.into())).unwrap();
                }
            }
            x => warn!("{x:?}"),
        }
    }

    async fn encapsulate_packet(&self, payload: &[u8]) -> Vec<u8> {
        let len = 148.max(payload.len() + 32);
        let mut dst = vec![0; len];
        let mut t = self.peer().await;
        let packet = t.encapsulate(payload, &mut dst);
        match packet {
            TunnResult::WriteToNetwork(p) => p.to_vec(),
            unexpected => {
                error!("{:?}", unexpected);
                vec![]
            }
        }
    }

    pub async fn peer(&self) -> tokio::sync::MutexGuard<'_, Tunn> {
        self.peer.lock().await
    }

    pub async fn new(
        peer_static_public: x25519_dalek::PublicKey,
        udp: Arc<UdpSocket>,
        peer_endpoint: SocketAddr,
        bus_tx: tokio::sync::broadcast::Sender<Event>,
    ) -> Self {
        let peer = Arc::new(Mutex::new(Self::create_tunnel(peer_static_public)));

        Self {
            source_peer_addr: Arc::new(RwLock::new(None)),
            peer,
            udp,
            peer_endpoint,
            bus_rx: bus_tx.subscribe(),
            bus_tx,
        }
    }

    fn create_tunnel(peer_static_public: x25519_dalek::PublicKey) -> Tunn {
        let secret_bytes: [u8; 32] = general_purpose::STANDARD
            .decode("AEqXrLFT4qjYq3wmX0456iv94uM6nDj5ugp6Jedcflg=")
            .unwrap()
            .try_into()
            .unwrap();

        let private_key = StaticSecret::try_from(secret_bytes).unwrap();

        Tunn::new(private_key, peer_static_public, None, None, 0, None).unwrap()
    }

    /// Encapsulates and sends an IP packet back to the WireGuard client.
    pub async fn send_ip_packet(&self, packet: &[u8]) -> anyhow::Result<()> {
        let mut send_buf = [0u8; MAX_PACKET];
        match self.peer().await.encapsulate(packet, &mut send_buf) {
            TunnResult::WriteToNetwork(packet) => {
                self.udp.send_to(packet, self.peer_endpoint).await.unwrap();
                debug!(
                    "Sent {} bytes to WireGuard endpoint (encrypted IP packet)",
                    packet.len()
                );
            }
            TunnResult::Err(e) => {
                error!("Failed to encapsulate IP packet: {:?}", e);
            }
            TunnResult::Done => {
                // Ignored
            }
            other => {
                error!(
                    "Unexpected WireGuard state during encapsulation: {:?}",
                    other
                );
            }
        };
        Ok(())
    }

    #[async_recursion]
    async fn handle_routine_tun_result<'a: 'async_recursion>(&self, result: TunnResult<'a>) -> () {
        match result {
            TunnResult::WriteToNetwork(packet) => {
                info!(
                    "Sending routine packet of {} bytes to WireGuard endpoint",
                    packet.len()
                );
                match self.udp.send_to(packet, self.peer_endpoint).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!(
                            "Failed to send routine packet to WireGuard endpoint: {:?}",
                            e
                        );
                    }
                };
            }
            TunnResult::Err(WireGuardError::ConnectionExpired) => {
                warn!("Wireguard handshake has expired!");

                let mut buf = vec![0u8; MAX_PACKET];
                let result = self
                    .peer()
                    .await
                    .format_handshake_initiation(&mut buf[..], false);

                self.handle_routine_tun_result(result).await
            }
            TunnResult::Err(e) => {
                error!(
                    "Failed to prepare routine packet for WireGuard endpoint: {:?}",
                    e
                );
            }
            TunnResult::Done => {
                // Sleep for a bit
                // tokio::time::sleep(Duration::from_millis(1)).await;
            }
            other => {
                warn!("Unexpected WireGuard routine task state: {:?}", other);
            }
        };
    }

    // fn route_protocol(&self, packet: &[u8]) -> Option<Protocol> {
    //     match IpVersion::of_packet(packet) {
    //         Ok(IpVersion::Ipv4) => Ipv4Packet::new_checked(&packet)
    //             .ok()
    //             // Only care if the packet is destined for this tunnel
    //             .filter(|packet| Ipv4Addr::from(packet.dst_addr()) == self.source_peer_ip)
    //             .and_then(|packet| match packet.next_header() {
    //                 IpProtocol::Tcp => Some(Protocol::Tcp),
    //                 IpProtocol::Udp => Some(Protocol::Udp),
    //                 // Unrecognized protocol, so we cannot determine where to route
    //                 _ => None,
    //             }),
    //         Ok(IpVersion::Ipv6) => Ipv6Packet::new_checked(&packet)
    //             .ok()
    //             // Only care if the packet is destined for this tunnel
    //             .filter(|packet| Ipv6Addr::from(packet.dst_addr()) == self.source_peer_ip)
    //             .and_then(|packet| match packet.next_header() {
    //                 IpProtocol::Tcp => Some(Protocol::Tcp),
    //                 IpProtocol::Udp => Some(Protocol::Udp),
    //                 // Unrecognized protocol, so we cannot determine where to route
    //                 _ => None,
    //             }),
    //         _ => None,
    //     }
    // }
}
