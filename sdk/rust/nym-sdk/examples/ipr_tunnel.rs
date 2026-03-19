// Integration test / example for IpMixStream.
//
// Connects to an IPR exit gateway through the mixnet, sends ICMP pings
// to 8.8.8.8, and verifies that echo replies come back.
//
// Usage:
//   cargo run --example ipr_tunnel
//   cargo run --example ipr_tunnel -- --gateway <RECIPIENT_ADDRESS>

use std::net::Ipv4Addr;
use std::time::Duration;

use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::ipr_wrapper::{IpMixStream, NetworkEnvironment};
use pnet_packet::icmp::echo_reply::EchoReplyPacket;
use pnet_packet::icmp::echo_request::{EchoRequestPacket, MutableEchoRequestPacket};
use pnet_packet::icmp::{IcmpPacket, IcmpTypes};
use pnet_packet::ipv4::{Ipv4Flags, Ipv4Packet, MutableIpv4Packet};
use pnet_packet::Packet;

const PING_TARGET: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
const PING_COUNT: u16 = 5;
const IDENTIFIER: u16 = 0x2119;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    let args: Vec<String> = std::env::args().collect();
    let gateway_addr = args
        .iter()
        .position(|a| a == "--gateway")
        .and_then(|i| args.get(i + 1));

    let mut tunnel = if let Some(addr) = gateway_addr {
        let recipient = addr.parse().expect("invalid Recipient address");
        IpMixStream::new_with_gateway(NetworkEnvironment::Mainnet, recipient).await?
    } else {
        IpMixStream::new(NetworkEnvironment::Mainnet).await?
    };

    let source_ip = tunnel.allocated_ips().ipv4;
    println!("Tunnel up — IPv4: {source_ip}");
    println!("Nym address: {}", tunnel.nym_address());

    // Send ICMP echo requests
    for seq in 0..PING_COUNT {
        let icmp = create_icmpv4_echo_request(seq, IDENTIFIER)?;
        let ip_packet = wrap_icmp_in_ipv4(icmp, source_ip, PING_TARGET)?;
        let bundled = MultiIpPacketCodec::bundle_one_packet(ip_packet.packet().to_vec().into());
        tunnel.send_ip_packet(&bundled).await?;
        println!("Sent ping {seq} → {PING_TARGET}");
    }

    // Collect replies
    let mut replies = 0u16;
    let deadline = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => break,
            result = tunnel.handle_incoming() => {
                for pkt in result? {
                    if let Some((id, seq)) = parse_echo_reply(&pkt, source_ip) {
                        if id == IDENTIFIER {
                            replies += 1;
                            println!("Got reply for seq {seq} ({replies}/{PING_COUNT})");
                        }
                    }
                }
                if replies >= PING_COUNT {
                    break;
                }
            }
        }
    }

    println!("\nResult: {replies}/{PING_COUNT} replies received");
    if replies == 0 {
        println!("FAIL — no replies");
    } else {
        println!("OK");
    }

    tunnel.disconnect().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// ICMP helpers (from the nym-vpn-client helper crates, adapted for examples)
// ---------------------------------------------------------------------------

fn create_icmpv4_echo_request(
    sequence_number: u16,
    identifier: u16,
) -> anyhow::Result<EchoRequestPacket<'static>> {
    let buffer = vec![0u8; 64];
    let mut echo = MutableEchoRequestPacket::owned(buffer)
        .ok_or_else(|| anyhow::anyhow!("failed to create ICMP echo request"))?;

    echo.set_identifier(identifier);
    echo.set_sequence_number(sequence_number);
    echo.set_icmp_type(IcmpTypes::EchoRequest);
    echo.set_icmp_code(pnet_packet::icmp::IcmpCode::new(0));

    let checksum = pnet_packet::icmp::checksum(
        &IcmpPacket::new(echo.packet())
            .ok_or_else(|| anyhow::anyhow!("failed to create ICMP packet for checksum"))?,
    );
    echo.set_checksum(checksum);

    Ok(echo.consume_to_immutable())
}

fn wrap_icmp_in_ipv4(
    icmp: EchoRequestPacket,
    source: Ipv4Addr,
    destination: Ipv4Addr,
) -> anyhow::Result<Ipv4Packet<'static>> {
    let total_length = 20 + icmp.packet().len();
    let buffer = vec![0u8; total_length];
    let mut ip = MutableIpv4Packet::owned(buffer)
        .ok_or_else(|| anyhow::anyhow!("failed to create IPv4 packet"))?;

    ip.set_version(4);
    ip.set_header_length(5);
    ip.set_total_length(total_length as u16);
    ip.set_ttl(64);
    ip.set_next_level_protocol(pnet_packet::ip::IpNextHeaderProtocols::Icmp);
    ip.set_source(source);
    ip.set_destination(destination);
    ip.set_flags(Ipv4Flags::DontFragment);
    ip.set_payload(icmp.packet());
    ip.set_checksum(0);
    ip.set_checksum(ipv4_checksum(&ip.to_immutable()));

    Ok(ip.consume_to_immutable())
}

fn ipv4_checksum(header: &Ipv4Packet) -> u16 {
    let len = header.get_header_length() as usize * 2;
    let mut sum = 0u32;
    for i in 0..len {
        let word = ((header.packet()[2 * i] as u32) << 8) | header.packet()[2 * i + 1] as u32;
        sum += word;
    }
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}

fn parse_echo_reply(data: &[u8], expected_dst: Ipv4Addr) -> Option<(u16, u16)> {
    let ip = Ipv4Packet::new(data)?;
    if ip.get_destination() != expected_dst {
        return None;
    }
    let icmp = IcmpPacket::new(ip.payload())?;
    if icmp.get_icmp_type() != IcmpTypes::EchoReply {
        return None;
    }
    let reply = EchoReplyPacket::new(icmp.packet())?;
    Some((reply.get_identifier(), reply.get_sequence_number()))
}
