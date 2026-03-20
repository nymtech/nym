// Smoke test for IpMixStream: connect to an IPR, send a ping, check we get a reply.
//
// Usage:
//   cargo run --example ipr_tunnel
//   cargo run --example ipr_tunnel -- --gateway <RECIPIENT_ADDRESS>

use std::net::Ipv4Addr;
use std::time::Duration;

use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::ipr_wrapper::{IpMixStream, NetworkEnvironment};
use pnet_packet::icmp::echo_reply::EchoReplyPacket;
use pnet_packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet_packet::icmp::{IcmpPacket, IcmpTypes};
use pnet_packet::ipv4::{Ipv4Flags, MutableIpv4Packet};
use pnet_packet::Packet;

const PING_TARGET: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);

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

    let packet = build_icmp_ping(source_ip, PING_TARGET, 0)?;
    let bundled = MultiIpPacketCodec::bundle_one_packet(packet.into());
    tunnel.send_ip_packet(&bundled).await?;
    println!("Sent ping → {PING_TARGET}");

    let deadline = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => {
                println!("FAIL — no reply within 30s");
                break;
            }
            result = tunnel.handle_incoming() => {
                for pkt in result? {
                    if is_echo_reply(&pkt, source_ip) {
                        println!("OK — got echo reply");
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

// ICMP helpers (from nym-vpn-client helper crates, adapted for this example)

fn build_icmp_ping(src: Ipv4Addr, dst: Ipv4Addr, seq: u16) -> anyhow::Result<Vec<u8>> {
    // ICMP echo request
    let mut echo = MutableEchoRequestPacket::owned(vec![0u8; 64])
        .ok_or_else(|| anyhow::anyhow!("failed to create ICMP packet"))?;
    echo.set_icmp_type(IcmpTypes::EchoRequest);
    echo.set_icmp_code(pnet_packet::icmp::IcmpCode::new(0));
    echo.set_sequence_number(seq);
    let cksum = pnet_packet::icmp::checksum(
        &IcmpPacket::new(echo.packet()).ok_or_else(|| anyhow::anyhow!("checksum failed"))?,
    );
    echo.set_checksum(cksum);

    // IPv4 wrapper
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

fn is_echo_reply(data: &[u8], expected_dst: Ipv4Addr) -> bool {
    let Some(ip) = pnet_packet::ipv4::Ipv4Packet::new(data) else {
        return false;
    };
    if ip.get_destination() != expected_dst {
        return false;
    }
    let Some(reply) = EchoReplyPacket::new(ip.payload()) else {
        return false;
    };
    reply.get_icmp_type() == IcmpTypes::EchoReply
}
