//! DNS lookup through the Nym mixnet.
//!
//! Resolves `example.com` via clearnet (hickory-resolver) and via the mixnet
//! (hickory-proto UDP query to Cloudflare 1.1.1.1), then compares resolved
//! IPs and timing.
//!
//! Run with:
//!   cargo run -p smolmix --example udp
//!   cargo run -p smolmix --example udp -- --ipr <IPR_ADDRESS>

use std::net::Ipv4Addr;

use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use hickory_resolver::TokioResolver;
use smolmix::Tunnel;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();

    let domain = "example.com";

    // Clearnet baseline via hickory-resolver
    info!("Clearnet DNS lookup for '{domain}'...");
    let resolver = TokioResolver::builder_tokio()?.build();
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

    // Mixnet: hickory-proto query over smolmix UDP
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

    let mut query = Message::new();
    query.set_recursion_desired(true);
    query.add_query(Query::query(Name::from_ascii(domain)?, RecordType::A));
    let query_bytes = query.to_vec()?;

    // UDP is connectionless — no setup phase, just send/recv
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
        .answers()
        .iter()
        .filter_map(|r| match r.data() {
            RData::A(a) => Some(a.0),
            _ => None,
        })
        .collect();

    // Results
    info!("Clearnet: {:?} ({:?})", clearnet_ips, clearnet_duration);
    info!("Mixnet:   {:?} ({:?})", mixnet_ips, mixnet_duration);

    let ips_match = !mixnet_ips.is_empty() && mixnet_ips.iter().all(|ip| clearnet_ips.contains(ip));
    info!("IPs match: {ips_match}");

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
