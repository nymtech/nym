// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Smoke test for IpMixStream: connect to an IPR, send a ping, check we get a reply.
// Tests both IPv4 and IPv6 paths.
//
// Usage:
//   cargo run --example ipr_tunnel
//   cargo run --example ipr_tunnel -- --ipr <IPR_ADDRESS>
//
// e.g. cargo run --example ipr_tunnel -- --ipr 6B6iuWX4bQP4GVA4Yq7XmZencaaGw6BaPY6xJWYSwsbF.6g6LRx1fgU2Q2A4ZPKonYHtfBARh1GPMe1LtXk6vpRR8@q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1

use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::ipr_wrapper::IpMixStream;
use pnet_packet::icmp::echo_reply::EchoReplyPacket;
use pnet_packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet_packet::icmp::{IcmpPacket, IcmpTypes};
use pnet_packet::icmpv6::Icmpv6Types;
use pnet_packet::ipv4::{Ipv4Flags, MutableIpv4Packet};
use pnet_packet::ipv6::MutableIpv6Packet;
use pnet_packet::Packet;

const PING4_TARGET: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
const PING6_TARGET: Ipv6Addr = Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    let args: Vec<String> = std::env::args().collect();
    let ipr_addr = args
        .iter()
        .position(|a| a == "--ipr")
        .and_then(|i| args.get(i + 1));

    let mut tunnel = if let Some(addr) = ipr_addr {
        let recipient = addr.parse().expect("invalid IPR address");
        IpMixStream::new_with_ipr(recipient).await?
    } else {
        IpMixStream::new().await?
    };

    let ips = tunnel.allocated_ips();
    let src4 = ips.ipv4;
    let src6 = ips.ipv6;
    println!("Tunnel up — IPv4: {src4}, IPv6: {src6}");

    // Send IPv4 ping (ICMP seq=0, unrelated to LP Stream sequence numbers)
    let pkt4 = build_icmp_ping(src4, PING4_TARGET, 0)?;
    let bundled = MultiIpPacketCodec::bundle_one_packet(pkt4.into());
    tunnel.send_ip_packet(&bundled).await?;
    println!("Sent ping → {PING4_TARGET}");

    // Send IPv6 ping
    let pkt6 = build_icmpv6_ping(src6, PING6_TARGET, 0)?;
    let bundled = MultiIpPacketCodec::bundle_one_packet(pkt6.into());
    tunnel.send_ip_packet(&bundled).await?;
    println!("Sent ping → {PING6_TARGET}");

    let mut got_v4 = false;
    let mut got_v6 = false;
    let deadline = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => {
                if !got_v4 { println!("FAIL — no IPv4 reply within 30s"); }
                if !got_v6 { println!("FAIL — no IPv6 reply within 30s"); }
                break;
            }
            result = tunnel.handle_incoming() => {
                for pkt in result? {
                    if !got_v4 && is_echo_reply_v4(&pkt, src4) {
                        println!("OK — got IPv4 echo reply");
                        got_v4 = true;
                    }
                    if !got_v6 && is_echo_reply_v6(&pkt, src6) {
                        println!("OK — got IPv6 echo reply");
                        got_v6 = true;
                    }
                    if got_v4 && got_v6 {
                        tunnel.disconnect().await;
                        return Ok(());
                    }
                }
            }
        }
    }

    tunnel.disconnect().await;
    Ok(())
}

// --- IPv4 ICMP helpers ---

fn build_icmp_ping(src: Ipv4Addr, dst: Ipv4Addr, seq: u16) -> anyhow::Result<Vec<u8>> {
    let mut echo = MutableEchoRequestPacket::owned(vec![0u8; 64])
        .ok_or_else(|| anyhow::anyhow!("failed to create ICMP packet"))?;
    echo.set_icmp_type(IcmpTypes::EchoRequest);
    echo.set_icmp_code(pnet_packet::icmp::IcmpCode::new(0));
    echo.set_sequence_number(seq);
    let cksum = pnet_packet::icmp::checksum(
        &IcmpPacket::new(echo.packet()).ok_or_else(|| anyhow::anyhow!("checksum failed"))?,
    );
    echo.set_checksum(cksum);

    let total_len = 20 + echo.packet().len();
    let mut ip = MutableIpv4Packet::owned(vec![0u8; total_len])
        .ok_or_else(|| anyhow::anyhow!("failed to create IPv4 packet"))?;
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
    Ok(buf)
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

fn is_echo_reply_v4(data: &[u8], expected_dst: Ipv4Addr) -> bool {
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
    if reply.get_icmp_type() != IcmpTypes::EchoReply {
        return false;
    }
    println!(
        "  IPv4 reply: {} → {}, seq={}",
        ip.get_source(),
        ip.get_destination(),
        reply.get_sequence_number(),
    );
    true
}

// --- IPv6 ICMPv6 helpers ---

fn build_icmpv6_ping(src: Ipv6Addr, dst: Ipv6Addr, seq: u16) -> anyhow::Result<Vec<u8>> {
    let mut echo =
        pnet_packet::icmpv6::echo_request::MutableEchoRequestPacket::owned(vec![0u8; 64])
            .ok_or_else(|| anyhow::anyhow!("failed to create ICMPv6 packet"))?;
    echo.set_icmpv6_type(Icmpv6Types::EchoRequest);
    echo.set_icmpv6_code(pnet_packet::icmpv6::Icmpv6Code::new(0));
    echo.set_sequence_number(seq);
    let cksum = pnet_packet::icmpv6::checksum(
        &pnet_packet::icmpv6::Icmpv6Packet::new(echo.packet())
            .ok_or_else(|| anyhow::anyhow!("checksum failed"))?,
        &src,
        &dst,
    );
    echo.set_checksum(cksum);

    let payload_len = echo.packet().len();
    let mut ip = MutableIpv6Packet::owned(vec![0u8; 40 + payload_len])
        .ok_or_else(|| anyhow::anyhow!("failed to create IPv6 packet"))?;
    ip.set_version(6);
    ip.set_payload_length(payload_len as u16);
    ip.set_next_header(pnet_packet::ip::IpNextHeaderProtocols::Icmpv6);
    ip.set_hop_limit(64);
    ip.set_source(src);
    ip.set_destination(dst);
    ip.set_payload(echo.packet());

    Ok(ip.consume_to_immutable().packet().to_vec())
}

fn is_echo_reply_v6(data: &[u8], expected_dst: Ipv6Addr) -> bool {
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
    if reply.get_icmpv6_type() != Icmpv6Types::EchoReply {
        return false;
    }
    println!(
        "  IPv6 reply: {} → {}, seq={}",
        ip.get_source(),
        ip.get_destination(),
        reply.get_sequence_number(),
    );
    true
}
