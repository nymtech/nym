// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Smoke test for `IpMixStream`: connect to an IPR, send a ping, check we get a reply.
//!
//! Tests both IPv4 and IPv6 paths.
//!
//! Run with: `cargo run --example ipr_tunnel -- --ipr <IPR_ADDRESS>`

use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_ip_packet_requests::icmp_utils::{
    build_icmp_ping, build_icmpv6_ping, is_echo_reply_v4, is_echo_reply_v6,
};
use nym_sdk::ipr_wrapper::IpMixStream;

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
    let pkt4 = build_icmp_ping(src4, PING4_TARGET, 0).expect("failed to build ICMP packet");
    let bundled = MultiIpPacketCodec::bundle_one_packet(pkt4.into());
    tunnel.send_ip_packet(&bundled).await?;
    println!("Sent ping → {PING4_TARGET}");

    // Send IPv6 ping
    let pkt6 = build_icmpv6_ping(src6, PING6_TARGET, 0).expect("failed to build ICMPv6 packet");
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
