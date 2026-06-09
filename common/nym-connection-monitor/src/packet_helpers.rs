// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::{Ipv4Addr, Ipv6Addr};

use bytes::Bytes;
use pnet_packet::{
    Packet,
    icmp::{
        IcmpPacket,
        echo_reply::EchoReplyPacket,
        echo_request::{EchoRequestPacket, MutableEchoRequestPacket},
    },
    icmpv6,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
    ipv6::{Ipv6Packet, MutableIpv6Packet},
};

use crate::error::{Error, Result};

pub fn create_icmpv4_echo_request(
    sequence_number: u16,
    identifier: u16,
) -> Result<EchoRequestPacket<'static>> {
    let buffer = vec![0; 64];
    let mut icmp_echo_request = MutableEchoRequestPacket::owned(buffer)
        .ok_or(Error::IcmpEchoRequestPacketCreationFailure)?;

    // Configure the ICMP echo request packet
    icmp_echo_request.set_identifier(identifier);
    icmp_echo_request.set_sequence_number(sequence_number);
    icmp_echo_request.set_icmp_type(pnet_packet::icmp::IcmpTypes::EchoRequest);
    icmp_echo_request.set_icmp_code(pnet_packet::icmp::IcmpCode::new(0));

    // Calculate checksum once we've set all the fields
    let icmp_packet =
        IcmpPacket::new(icmp_echo_request.packet()).ok_or(Error::IcmpPacketCreationFailure)?;
    let checksum = pnet_packet::icmp::checksum(&icmp_packet);
    icmp_echo_request.set_checksum(checksum);

    Ok(icmp_echo_request.consume_to_immutable())
}

pub fn create_icmpv6_echo_request(
    sequence_number: u16,
    identifier: u16,
    source: &Ipv6Addr,
    destination: &Ipv6Addr,
) -> Result<icmpv6::echo_request::EchoRequestPacket<'static>> {
    let buffer = vec![0; 64];
    // let mut icmp_echo_request = MutableEchoRequestPacket::owned(buffer)
    let mut icmp_echo_request = icmpv6::echo_request::MutableEchoRequestPacket::owned(buffer)
        .ok_or(Error::IcmpEchoRequestPacketCreationFailure)?;

    // Configure the ICMP echo request packet
    icmp_echo_request.set_identifier(identifier);
    icmp_echo_request.set_sequence_number(sequence_number);
    icmp_echo_request.set_icmpv6_type(pnet_packet::icmpv6::Icmpv6Types::EchoRequest);
    icmp_echo_request.set_icmpv6_code(pnet_packet::icmpv6::Icmpv6Code::new(0));

    // Calculate checksum once we've set all the fields
    let icmp_packet = icmpv6::Icmpv6Packet::new(icmp_echo_request.packet())
        .ok_or(Error::IcmpPacketCreationFailure)?;
    let checksum = pnet_packet::icmpv6::checksum(&icmp_packet, source, destination);
    icmp_echo_request.set_checksum(checksum);

    Ok(icmp_echo_request.consume_to_immutable())
}

pub fn wrap_icmp_in_ipv4(
    icmp_echo_request: EchoRequestPacket,
    source: Ipv4Addr,
    destination: Ipv4Addr,
) -> Result<Ipv4Packet> {
    // 20 bytes for IPv4 header + ICMP payload
    let total_length = 20 + icmp_echo_request.packet().len();
    // IPv4 header + ICMP payload
    let ipv4_buffer = vec![0u8; 20 + icmp_echo_request.packet().len()];
    let mut ipv4_packet =
        MutableIpv4Packet::owned(ipv4_buffer).ok_or(Error::Ipv4PacketCreationFailure)?;

    let total_length_u16 =
        u16::try_from(total_length).map_err(|_| Error::PacketLengthOverflow {
            length: total_length,
        })?;

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(total_length_u16);
    ipv4_packet.set_ttl(64);
    ipv4_packet.set_next_level_protocol(pnet_packet::ip::IpNextHeaderProtocols::Icmp);
    ipv4_packet.set_source(source);
    ipv4_packet.set_destination(destination);
    ipv4_packet.set_flags(pnet_packet::ipv4::Ipv4Flags::DontFragment);
    ipv4_packet.set_checksum(0);
    ipv4_packet.set_payload(icmp_echo_request.packet());

    let ipv4_checksum = compute_ipv4_checksum(&ipv4_packet.to_immutable());
    ipv4_packet.set_checksum(ipv4_checksum);

    Ok(ipv4_packet.consume_to_immutable())
}

pub fn wrap_icmp_in_ipv6(
    icmp_echo_request: icmpv6::echo_request::EchoRequestPacket,
    source: Ipv6Addr,
    destination: Ipv6Addr,
) -> Result<Ipv6Packet> {
    let payload_length = icmp_echo_request.packet().len();
    let payload_length_u16 =
        u16::try_from(payload_length).map_err(|_| Error::PacketLengthOverflow {
            length: payload_length,
        })?;

    let ipv6_buffer = vec![0u8; 40 + payload_length];
    let mut ipv6_packet =
        MutableIpv6Packet::owned(ipv6_buffer).ok_or(Error::Ipv4PacketCreationFailure)?;

    ipv6_packet.set_version(6);
    ipv6_packet.set_payload_length(payload_length_u16);
    ipv6_packet.set_next_header(pnet_packet::ip::IpNextHeaderProtocols::Icmpv6);
    ipv6_packet.set_hop_limit(64);
    ipv6_packet.set_source(source);
    ipv6_packet.set_destination(destination);
    ipv6_packet.set_payload(icmp_echo_request.packet());

    Ok(ipv6_packet.consume_to_immutable())
}

// Compute IPv4 checksum: sum all 16-bit words, add carry, take one's complement
pub(crate) fn compute_ipv4_checksum(header: &Ipv4Packet) -> u16 {
    // Header length in 16-bit words
    let len = header.get_header_length() as usize * 2;
    let mut sum = 0u32;

    for i in 0..len {
        let word = ((header.packet()[2 * i] as u32) << 8) | header.packet()[2 * i + 1] as u32;
        sum += word;
    }

    // Add the carry
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // One's complement
    !sum as u16
}

pub(crate) fn is_icmp_echo_reply(packet: &Bytes) -> Option<(u16, Ipv4Addr, Ipv4Addr)> {
    if let Some(ipv4_packet) = Ipv4Packet::new(packet)
        && let Some(icmp_packet) = IcmpPacket::new(ipv4_packet.payload())
        && let Some(echo_reply) = EchoReplyPacket::new(icmp_packet.packet())
    {
        return Some((
            echo_reply.get_identifier(),
            ipv4_packet.get_source(),
            ipv4_packet.get_destination(),
        ));
    }
    None
}

pub(crate) fn is_icmp_v6_echo_reply(packet: &Bytes) -> Option<(u16, Ipv6Addr, Ipv6Addr)> {
    if let Some(ipv6_packet) = Ipv6Packet::new(packet)
        && let Some(icmp_packet) = IcmpPacket::new(ipv6_packet.payload())
        && let Some(echo_reply) =
            pnet_packet::icmpv6::echo_reply::EchoReplyPacket::new(icmp_packet.packet())
    {
        return Some((
            echo_reply.get_identifier(),
            ipv6_packet.get_source(),
            ipv6_packet.get_destination(),
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet_packet::icmp::IcmpTypes;
    use pnet_packet::icmpv6::Icmpv6Types;
    use pnet_packet::ip::IpNextHeaderProtocols;

    const V4_SRC: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);
    const V4_DST: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 2);
    const V6_SRC: Ipv6Addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    const V6_DST: Ipv6Addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);

    #[test]
    fn icmpv4_echo_request_sets_fields_and_valid_checksum() {
        let echo = create_icmpv4_echo_request(42, 7).unwrap();
        assert_eq!(echo.get_sequence_number(), 42);
        assert_eq!(echo.get_identifier(), 7);
        assert_eq!(echo.get_icmp_type(), IcmpTypes::EchoRequest);

        // pnet's `checksum` skips the checksum word, so recomputing on the produced
        // packet must equal the stored value.
        let icmp = IcmpPacket::new(echo.packet()).unwrap();
        assert_eq!(echo.get_checksum(), pnet_packet::icmp::checksum(&icmp));
    }

    #[test]
    fn icmpv6_echo_request_sets_fields_and_valid_checksum() {
        let echo = create_icmpv6_echo_request(99, 12, &V6_SRC, &V6_DST).unwrap();
        assert_eq!(echo.get_sequence_number(), 99);
        assert_eq!(echo.get_identifier(), 12);
        assert_eq!(echo.get_icmpv6_type(), Icmpv6Types::EchoRequest);

        let icmpv6 = icmpv6::Icmpv6Packet::new(echo.packet()).unwrap();
        assert_eq!(
            echo.get_checksum(),
            pnet_packet::icmpv6::checksum(&icmpv6, &V6_SRC, &V6_DST)
        );
    }

    #[test]
    fn wrap_icmp_in_ipv4_sets_headers_and_payload() {
        let echo = create_icmpv4_echo_request(1, 2).unwrap();
        let echo_bytes = echo.packet().to_vec();
        let packet = wrap_icmp_in_ipv4(echo, V4_SRC, V4_DST).unwrap();

        assert_eq!(packet.get_version(), 4);
        assert_eq!(packet.get_header_length(), 5);
        assert_eq!(packet.get_total_length() as usize, 20 + echo_bytes.len());
        assert_eq!(packet.get_ttl(), 64);
        assert_eq!(
            packet.get_next_level_protocol(),
            IpNextHeaderProtocols::Icmp
        );
        assert_eq!(packet.get_source(), V4_SRC);
        assert_eq!(packet.get_destination(), V4_DST);
        assert_eq!(packet.payload(), echo_bytes.as_slice());
    }

    #[test]
    fn wrap_icmp_in_ipv6_sets_headers_and_payload() {
        let echo = create_icmpv6_echo_request(1, 2, &V6_SRC, &V6_DST).unwrap();
        let echo_bytes = echo.packet().to_vec();
        let packet = wrap_icmp_in_ipv6(echo, V6_SRC, V6_DST).unwrap();

        assert_eq!(packet.get_version(), 6);
        assert_eq!(packet.get_payload_length() as usize, echo_bytes.len());
        assert_eq!(packet.get_next_header(), IpNextHeaderProtocols::Icmpv6);
        assert_eq!(packet.get_hop_limit(), 64);
        assert_eq!(packet.get_source(), V6_SRC);
        assert_eq!(packet.get_destination(), V6_DST);
        assert_eq!(packet.payload(), echo_bytes.as_slice());
    }

    #[test]
    fn compute_ipv4_checksum_is_zero_on_correctly_checksummed_packet() {
        let echo = create_icmpv4_echo_request(1, 2).unwrap();
        let packet = wrap_icmp_in_ipv4(echo, V4_SRC, V4_DST).unwrap();
        // RFC 1071: summing every 16-bit word of a header that already contains its
        // own checksum yields all-ones; the one's complement is therefore zero.
        assert_eq!(compute_ipv4_checksum(&packet), 0);
    }

    #[test]
    fn is_icmp_echo_reply_extracts_identifier_and_addresses() {
        // pnet's EchoReply/EchoRequest share the same byte layout (only the ICMP
        // type field differs) and `is_icmp_echo_reply` does not check the type,
        // so a wrapped echo *request* exercises the same parsing path.
        let identifier = 1234;
        let echo = create_icmpv4_echo_request(7, identifier).unwrap();
        let packet = wrap_icmp_in_ipv4(echo, V4_SRC, V4_DST).unwrap();
        let bytes = Bytes::copy_from_slice(packet.packet());

        assert_eq!(
            is_icmp_echo_reply(&bytes),
            Some((identifier, V4_SRC, V4_DST))
        );
    }

    #[test]
    fn is_icmp_v6_echo_reply_extracts_identifier_and_addresses() {
        let identifier = 5678;
        let echo = create_icmpv6_echo_request(7, identifier, &V6_SRC, &V6_DST).unwrap();
        let packet = wrap_icmp_in_ipv6(echo, V6_SRC, V6_DST).unwrap();
        let bytes = Bytes::copy_from_slice(packet.packet());

        assert_eq!(
            is_icmp_v6_echo_reply(&bytes),
            Some((identifier, V6_SRC, V6_DST))
        );
    }

    #[test]
    fn is_icmp_echo_reply_returns_none_for_undersized_bytes() {
        let bytes = Bytes::from_static(&[0u8; 4]);
        assert!(is_icmp_echo_reply(&bytes).is_none());
        assert!(is_icmp_v6_echo_reply(&bytes).is_none());
    }
}
