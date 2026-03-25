// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Extracted from sdk/rust/nym-sdk/examples/ipr_tunnel.rs

//! ICMP/ICMPv6 packet construction and reply detection helpers for testing
//! IPR connectivity. Gated behind the `test-utils` feature.

use std::net::{Ipv4Addr, Ipv6Addr};

use pnet_packet::Packet;
use pnet_packet::icmp::echo_reply::EchoReplyPacket;
use pnet_packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet_packet::icmp::{IcmpPacket, IcmpTypes};
use pnet_packet::icmpv6::Icmpv6Types;
use pnet_packet::ipv4::{Ipv4Flags, MutableIpv4Packet};
use pnet_packet::ipv6::MutableIpv6Packet;

/// Build a complete IPv4 ICMP echo request packet.
pub fn build_icmp_ping(src: Ipv4Addr, dst: Ipv4Addr, seq: u16) -> Option<Vec<u8>> {
    let mut echo = MutableEchoRequestPacket::owned(vec![0u8; 64])?;
    echo.set_icmp_type(IcmpTypes::EchoRequest);
    echo.set_icmp_code(pnet_packet::icmp::IcmpCode::new(0));
    echo.set_sequence_number(seq);
    let cksum = pnet_packet::icmp::checksum(&IcmpPacket::new(echo.packet())?);
    echo.set_checksum(cksum);

    let total_len = 20 + echo.packet().len();
    let mut ip = MutableIpv4Packet::owned(vec![0u8; total_len])?;
    ip.set_version(4);
    ip.set_header_length(5);
    ip.set_total_length(total_len as u16);
    ip.set_ttl(64);
    ip.set_next_level_protocol(pnet_packet::ip::IpNextHeaderProtocols::Icmp);
    ip.set_source(src);
    ip.set_destination(dst);
    ip.set_flags(Ipv4Flags::DontFragment);
    ip.set_payload(echo.packet());

    let mut buf = ip.consume_to_immutable().packet().to_vec();
    let cksum = ipv4_checksum(&buf);
    buf[10] = (cksum >> 8) as u8;
    buf[11] = cksum as u8;
    Some(buf)
}

/// Build a complete IPv6 ICMPv6 echo request packet.
pub fn build_icmpv6_ping(src: Ipv6Addr, dst: Ipv6Addr, seq: u16) -> Option<Vec<u8>> {
    let mut echo =
        pnet_packet::icmpv6::echo_request::MutableEchoRequestPacket::owned(vec![0u8; 64])?;
    echo.set_icmpv6_type(Icmpv6Types::EchoRequest);
    echo.set_icmpv6_code(pnet_packet::icmpv6::Icmpv6Code::new(0));
    echo.set_sequence_number(seq);
    let cksum = pnet_packet::icmpv6::checksum(
        &pnet_packet::icmpv6::Icmpv6Packet::new(echo.packet())?,
        &src,
        &dst,
    );
    echo.set_checksum(cksum);

    let payload_len = echo.packet().len();
    let mut ip = MutableIpv6Packet::owned(vec![0u8; 40 + payload_len])?;
    ip.set_version(6);
    ip.set_payload_length(payload_len as u16);
    ip.set_next_header(pnet_packet::ip::IpNextHeaderProtocols::Icmpv6);
    ip.set_hop_limit(64);
    ip.set_source(src);
    ip.set_destination(dst);
    ip.set_payload(echo.packet());

    Some(ip.consume_to_immutable().packet().to_vec())
}

/// Check if a raw packet is an IPv4 ICMP echo reply destined to `expected_dst`.
pub fn is_echo_reply_v4(data: &[u8], expected_dst: Ipv4Addr) -> bool {
    let Some(ip) = pnet_packet::ipv4::Ipv4Packet::new(data) else {
        return false;
    };
    if ip.get_destination() != expected_dst {
        return false;
    }
    if ip.get_next_level_protocol() != pnet_packet::ip::IpNextHeaderProtocols::Icmp {
        return false;
    }
    let Some(reply) = EchoReplyPacket::new(ip.payload()) else {
        return false;
    };
    reply.get_icmp_type() == IcmpTypes::EchoReply
}

/// Check if a raw packet is an IPv6 ICMPv6 echo reply destined to `expected_dst`.
pub fn is_echo_reply_v6(data: &[u8], expected_dst: Ipv6Addr) -> bool {
    let Some(ip) = pnet_packet::ipv6::Ipv6Packet::new(data) else {
        return false;
    };
    if ip.get_destination() != expected_dst {
        return false;
    }
    if ip.get_next_header() != pnet_packet::ip::IpNextHeaderProtocols::Icmpv6 {
        return false;
    }
    let Some(reply) = pnet_packet::icmpv6::echo_reply::EchoReplyPacket::new(ip.payload()) else {
        return false;
    };
    reply.get_icmpv6_type() == Icmpv6Types::EchoReply
}

fn ipv4_checksum(header: &[u8]) -> u16 {
    let mut sum = 0u32;
    for i in (0..20).step_by(2) {
        sum += ((header[i] as u32) << 8) | header[i + 1] as u32;
    }
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}
