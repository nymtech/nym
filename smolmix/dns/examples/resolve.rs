//! DNS resolution: clearnet vs Nym mixnet comparison.
//!
//! Resolves a hostname via clearnet (hickory-resolver) and via the mixnet
//! (smolmix-dns), then compares resolved IPs and timing.
//!
//! Run with:
//!   cargo run -p smolmix-dns --example resolve
//!   cargo run -p smolmix-dns --example resolve -- --ipr <IPR_ADDRESS>

use std::net::Ipv4Addr;

use hickory_resolver::TokioResolver;
use smolmix::{Recipient, Tunnel};
use smolmix_dns::Resolver;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();

    let domain = "example.com";

    // Clearnet baseline via hickory-resolver
    info!("Clearnet DNS lookup for '{domain}'...");
    let clearnet_resolver = TokioResolver::builder_tokio()?.build();
    let clearnet_start = tokio::time::Instant::now();
    let lookup = clearnet_resolver.lookup_ip(domain).await?;
    let clearnet_ips: Vec<Ipv4Addr> = lookup
        .iter()
        .filter_map(|ip| match ip {
            std::net::IpAddr::V4(v4) => Some(v4),
            _ => None,
        })
        .collect();
    let clearnet_duration = clearnet_start.elapsed();
    info!("Clearnet: {:?} in {:?}", clearnet_ips, clearnet_duration);

    // Mixnet via smolmix-dns
    let args: Vec<String> = std::env::args().collect();
    let ipr_addr = args
        .iter()
        .position(|a| a == "--ipr")
        .and_then(|i| args.get(i + 1));

    let tunnel = if let Some(addr) = ipr_addr {
        let recipient: Recipient = addr.parse().expect("invalid IPR address");
        Tunnel::new_with_ipr(recipient).await?
    } else {
        Tunnel::new().await?
    };

    let resolver = Resolver::new(&tunnel);

    // Full hickory API via Deref:
    let mixnet_start = tokio::time::Instant::now();
    let lookup = resolver.lookup_ip(domain).await?;
    let mixnet_ips: Vec<Ipv4Addr> = lookup
        .iter()
        .filter_map(|ip| match ip {
            std::net::IpAddr::V4(v4) => Some(v4),
            _ => None,
        })
        .collect();
    let mixnet_duration = mixnet_start.elapsed();

    // Convenience method for a second lookup:
    let addrs = resolver.resolve("nymtech.net", 443).await?;
    info!("Resolved nymtech.net:443 → {addrs:?}");

    // Compare
    info!("Results");
    info!("Clearnet: {:?} ({:?})", clearnet_ips, clearnet_duration);
    info!("Mixnet:   {:?} ({:?})", mixnet_ips, mixnet_duration);

    let ips_match = !mixnet_ips.is_empty() && mixnet_ips.iter().all(|ip| clearnet_ips.contains(ip));
    info!("IPs match: {ips_match}");

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown:  {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
