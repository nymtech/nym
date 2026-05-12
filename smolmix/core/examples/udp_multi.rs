// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Multiple DNS lookups + NTP time sync through the Nym mixnet.
//!
//! Resolves several hostnames and syncs the clock via NTP, all over
//! mixnet UDP. Demonstrates timeout handling, socket reuse, and raw
//! protocol construction over smolmix's `UdpSocket`.
//!
//! ```text
//! DNS / NTP (application-layer UDP protocols)
//!   └─ smolmix::UdpSocket (UDP over mixnet)
//!        └─ smoltcp (userspace IP stack)
//!             └─ Nym mixnet → IPR exit gateway → internet
//! ```
//!
//! ## What this demonstrates
//!
//! - Multiple DNS queries over a single `UdpSocket`
//! - Timeout handling with `tokio::time::timeout` (essential for UDP)
//! - NTP time sync via a raw 48-byte UDP packet
//!
//! Compare with `udp.rs` which does a single DNS lookup with clearnet comparison.
//!
//! ```sh
//! cargo run -p smolmix --example udp_multi
//! cargo run -p smolmix --example udp_multi -- --ipr <IPR_ADDRESS>
//! ```

use std::net::Ipv4Addr;

use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use smolmix::Tunnel;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Hostnames to resolve via mixnet DNS.
const DNS_TARGETS: &[&str] = &["example.com", "cloudflare.com", "nymtech.net"];

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let args: Vec<String> = std::env::args().collect();
    let ipr_addr = args
        .iter()
        .position(|a| a == "--ipr")
        .and_then(|i| args.get(i + 1));

    // Create the tunnel
    let mut builder = Tunnel::builder();
    if let Some(addr) = ipr_addr {
        builder = builder.ipr_address(addr.parse().expect("invalid IPR address"));
    }
    let tunnel = builder.build().await?;
    println!(
        "Tunnel ready, allocated IP: {}",
        tunnel.allocated_ips().ipv4
    );

    // Multiple DNS lookups over one UdpSocket
    // Each query goes to Cloudflare DNS (1.1.1.1:53) through the mixnet.
    // The DNS server sees the IPR exit gateway's IP, not yours.
    println!("\nPrivate DNS Lookups (via mixnet UDP)\n");

    let udp = tunnel.udp_socket().await?;

    for host in DNS_TARGETS {
        let start = std::time::Instant::now();

        let mut query = Message::query();
        query.metadata.recursion_desired = true;
        query.add_query(Query::query(Name::from_ascii(host)?, RecordType::A));
        let query_bytes = query.to_vec()?;

        udp.send_to(&query_bytes, "1.1.1.1:53".parse()?).await?;

        let mut buf = vec![0u8; 1500];
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(15), udp.recv_from(&mut buf)).await;

        match result {
            Ok(Ok((n, _))) => {
                let rtt = start.elapsed();
                let response = Message::from_vec(&buf[..n])?;
                let ips: Vec<_> = response
                    .answers
                    .iter()
                    .filter_map(|r| match r.data {
                        RData::A(a) => Some(a.0.to_string()),
                        _ => None,
                    })
                    .collect();
                println!("{host:<16} → {} (rtt: {rtt:.1?})", ips.join(", "));
            }
            Ok(Err(e)) => println!("{host:<16} → ERROR: {e}"),
            Err(_) => println!("{host:<16} → TIMEOUT"),
        }
    }

    // NTP time sync via mixnet UDP
    // NTP uses a simple 48-byte request/response over UDP port 123.
    // We first resolve pool.ntp.org via the mixnet, then send the NTP request.
    println!("\nNTP Time Sync (via mixnet UDP)\n");

    let ntp_ip = resolve_dns(&tunnel, "pool.ntp.org").await?;
    println!("Resolved pool.ntp.org → {ntp_ip}");

    // NTP request: 48 bytes, LI=0 Version=4 Mode=3 (client)
    let mut ntp_req = [0u8; 48];
    ntp_req[0] = 0x23;

    let ntp_udp = tunnel.udp_socket().await?;
    let start = std::time::Instant::now();
    let ntp_dest: std::net::SocketAddr = (ntp_ip, 123).into();
    ntp_udp.send_to(&ntp_req, ntp_dest).await?;

    let mut buf = [0u8; 48];
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        ntp_udp.recv_from(&mut buf),
    )
    .await;

    match result {
        Ok(Ok((n, _))) if n >= 48 => {
            let rtt = start.elapsed();

            // Transmit timestamp at bytes 40..48 (seconds since 1900-01-01)
            let secs = u32::from_be_bytes([buf[40], buf[41], buf[42], buf[43]]);
            let frac = u32::from_be_bytes([buf[44], buf[45], buf[46], buf[47]]);

            // NTP epoch (1900) → Unix epoch (1970)
            // Valid for Era 0 (until 2036-02-07); Era 1 wraps secs to 0.
            const NTP_TO_UNIX: u64 = 2_208_988_800;
            let unix_secs = secs as u64 - NTP_TO_UNIX;
            let millis = (frac as u64 * 1000) >> 32;

            let dt =
                chrono::DateTime::from_timestamp(unix_secs as i64, (millis * 1_000_000) as u32)
                    .expect("valid timestamp");
            println!("NTP response in {rtt:.1?}");
            println!("Unix timestamp: {unix_secs}.{millis:03}");
            println!("UTC: {}", dt.format("%Y-%m-%d %H:%M:%S%.3f UTC"));
        }
        Ok(Ok((n, _))) => println!("Short response: {n} bytes (expected 48)"),
        Ok(Err(e)) => println!("ERROR: {e}"),
        Err(_) => println!("TIMEOUT (30s)"),
    }

    tunnel.shutdown().await;
    Ok(())
}

/// Resolve a hostname to an IPv4 address via mixnet UDP DNS.
async fn resolve_dns(tunnel: &Tunnel, host: &str) -> Result<Ipv4Addr, BoxError> {
    let mut query = Message::query();
    query.metadata.recursion_desired = true;
    query.add_query(Query::query(Name::from_ascii(host)?, RecordType::A));
    let query_bytes = query.to_vec()?;

    let udp = tunnel.udp_socket().await?;
    udp.send_to(&query_bytes, "1.1.1.1:53".parse()?).await?;

    let mut buf = vec![0u8; 1500];
    let (n, _) = udp.recv_from(&mut buf).await?;

    let response = Message::from_vec(&buf[..n])?;
    let ip = response
        .answers
        .iter()
        .find_map(|r| match r.data {
            RData::A(a) => Some(a.0),
            _ => None,
        })
        .ok_or("no A record in DNS response")?;
    Ok(ip)
}
