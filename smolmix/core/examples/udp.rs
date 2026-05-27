// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! DNS lookup through the Nym mixnet.
//!
//! Resolves `example.com` twice: once via the system path with
//! `hickory-resolver`, and once by sending a raw DNS query to Cloudflare's
//! 1.1.1.1 over a `smolmix::UdpSocket`. The resolved IPs and timings are
//! printed for comparison.
//!
//! ```text
//! DNS query / response (application-layer UDP)
//!   └─ smolmix::UdpSocket (UDP over mixnet)
//!        └─ smoltcp (userspace IP stack)
//!             └─ Nym mixnet → IPR exit gateway → internet
//! ```
//!
//! ```sh
//! cargo run -p smolmix --example udp
//! cargo run -p smolmix --example udp -- --ipr <IPR_ADDRESS>
//! ```

use std::net::Ipv4Addr;

use hickory_proto::{
    op::{Message, Query},
    rr::{Name, RData, RecordType},
};
use hickory_resolver::TokioResolver;
use smolmix::Tunnel;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();

    let domain = "example.com";

    info!("Clearnet DNS lookup for '{domain}'...");
    let resolver = TokioResolver::builder_tokio()?.build()?;
    let clearnet_start = tokio::time::Instant::now();
    let lookup = resolver.lookup_ip(domain).await?;
    let clearnet_ips: Vec<Ipv4Addr> = lookup
        .iter()
        .filter_map(|ip| match ip {
            std::net::IpAddr::V4(v4) => Some(v4),
            _ => None,
        })
        .collect();
    let clearnet_duration = clearnet_start.elapsed();
    info!("Clearnet: {:?} in {:?}", clearnet_ips, clearnet_duration);

    // hickory-proto (not hickory-resolver) so the raw UDP query goes through
    // the tunnel directly, instead of routing back to the system resolver
    // and out over clearnet.
    let args: Vec<String> = std::env::args().collect();
    let ipr_addr = args
        .iter()
        .position(|a| a == "--ipr")
        .and_then(|i| args.get(i + 1));

    let mut builder = Tunnel::builder();
    if let Some(addr) = ipr_addr {
        builder = builder.ipr_address(addr.parse().expect("invalid IPR address"));
    }
    let tunnel = builder.build().await?;

    let udp = tunnel.udp_socket().await?;

    let mut query = Message::query();
    query.metadata.recursion_desired = true;
    query.add_query(Query::query(Name::from_ascii(domain)?, RecordType::A));
    let query_bytes = query.to_vec()?;

    // UDP has no handshake; just send_to + recv_from.
    info!("Sending DNS query via mixnet...");
    let mixnet_start = tokio::time::Instant::now();
    udp.send_to(&query_bytes, "1.1.1.1:53".parse()?).await?;
    info!("Query sent ({:?})", mixnet_start.elapsed());

    let mut buf = vec![0u8; 1500];
    let (n, _from) = udp.recv_from(&mut buf).await?;
    let mixnet_duration = mixnet_start.elapsed();
    info!("Response received ({} bytes, {:?})", n, mixnet_duration);

    let response = Message::from_vec(&buf[..n])?;
    let mixnet_ips: Vec<Ipv4Addr> = response
        .answers
        .iter()
        .filter_map(|r| match r.data {
            RData::A(a) => Some(a.0),
            _ => None,
        })
        .collect();

    info!("Clearnet: {:?} ({:?})", clearnet_ips, clearnet_duration);
    info!("Mixnet: {:?} ({:?})", mixnet_ips, mixnet_duration);

    let ips_match = !mixnet_ips.is_empty() && mixnet_ips.iter().all(|ip| clearnet_ips.contains(ip));
    info!("IPs match: {ips_match}");

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
