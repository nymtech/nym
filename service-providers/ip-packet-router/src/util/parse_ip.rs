use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::error::IpPacketRouterError;

pub(crate) struct ParsedPacket<'a> {
    pub(crate) packet_type: &'a str,
    pub(crate) src_addr: IpAddr,
    pub(crate) dst_addr: IpAddr,
    pub(crate) dst: Option<SocketAddr>,
}

pub(crate) fn parse_packet(packet: &[u8]) -> Result<ParsedPacket, IpPacketRouterError> {
    let headers = etherparse::SlicedPacket::from_ip(packet).map_err(|err| {
        log::warn!("Unable to parse incoming data as IP packet: {err}");
        IpPacketRouterError::PacketParseFailed { source: err }
    })?;

    let (packet_type, dst_port) = match headers.transport {
        Some(etherparse::TransportSlice::Udp(header)) => ("udp", Some(header.destination_port())),
        Some(etherparse::TransportSlice::Tcp(header)) => ("tcp", Some(header.destination_port())),
        Some(etherparse::TransportSlice::Icmpv4(_)) => ("icmpv4", None),
        Some(etherparse::TransportSlice::Icmpv6(_)) => ("icmpv6", None),
        Some(etherparse::TransportSlice::Unknown(_)) => ("unknown", None),
        None => {
            log::warn!("Received packet missing transport header");
            return Err(IpPacketRouterError::PacketMissingTransportHeader);
        }
    };

    let (src_addr, dst_addr, dst) = match headers.ip {
        Some(etherparse::InternetSlice::Ipv4(ipv4_header, _)) => {
            let src_addr: IpAddr = ipv4_header.source_addr().into();
            let dst_addr: IpAddr = ipv4_header.destination_addr().into();
            let dst = dst_port.map(|port| SocketAddr::new(dst_addr, port));
            (src_addr, dst_addr, dst)
        }
        Some(etherparse::InternetSlice::Ipv6(ipv6_header, _)) => {
            let src_addr: IpAddr = ipv6_header.source_addr().into();
            let dst_addr: IpAddr = ipv6_header.destination_addr().into();
            let dst = dst_port.map(|port| SocketAddr::new(dst_addr, port));
            (src_addr, dst_addr, dst)
        }
        None => {
            log::warn!("Received packet missing IP header");
            return Err(IpPacketRouterError::PacketMissingIpHeader);
        }
    };
    Ok(ParsedPacket {
        packet_type,
        src_addr,
        dst_addr,
        dst,
    })
}

// Constants for IPv4 and IPv6 headers
const IPV4_DEST_ADDR_START: usize = 16;
const IPV4_DEST_ADDR_LEN: usize = 4;
const IPV6_DEST_ADDR_START: usize = 24;
const IPV6_DEST_ADDR_LEN: usize = 16;

// Only parse the destination address, for when we don't need the other stuff
pub(crate) fn parse_dst_addr(packet: &[u8]) -> Option<IpAddr> {
    let version = packet.first().map(|v| v >> 4)?;
    match version {
        4 => {
            // IPv4
            let addr_end = IPV4_DEST_ADDR_START + IPV4_DEST_ADDR_LEN;
            let addr_array: [u8; IPV4_DEST_ADDR_LEN] = packet
                .get(IPV4_DEST_ADDR_START..addr_end)?
                .try_into()
                .ok()?;
            Some(IpAddr::V4(Ipv4Addr::from(addr_array)))
        }
        6 => {
            // IPv6
            let addr_end = IPV6_DEST_ADDR_START + IPV6_DEST_ADDR_LEN;
            let addr_array: [u8; IPV6_DEST_ADDR_LEN] = packet
                .get(IPV6_DEST_ADDR_START..addr_end)?
                .try_into()
                .ok()?;
            Some(IpAddr::V6(Ipv6Addr::from(addr_array)))
        }
        _ => None, // Unknown IP version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_destination_from_ip_packet() {
        // Create packet
        let builder =
            etherparse::PacketBuilder::ipv4([192, 168, 1, 1], [192, 168, 1, 2], 20).udp(21, 1234);
        let payload = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut packet = Vec::<u8>::with_capacity(builder.size(payload.len()));
        builder.write(&mut packet, &payload).unwrap();

        let dst_addr = parse_dst_addr(&packet).unwrap();
        assert_eq!(dst_addr, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));
    }
}
