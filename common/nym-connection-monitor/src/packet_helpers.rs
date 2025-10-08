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

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(total_length as u16);
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
    let ipv6_buffer = vec![0u8; 40 + icmp_echo_request.packet().len()];
    let mut ipv6_packet =
        MutableIpv6Packet::owned(ipv6_buffer).ok_or(Error::Ipv4PacketCreationFailure)?;

    ipv6_packet.set_version(6);
    ipv6_packet.set_payload_length(icmp_echo_request.packet().len() as u16);
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
