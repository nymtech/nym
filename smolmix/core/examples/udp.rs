//! Raw UDP socket through the Nym mixnet.
//!
//! Demonstrates using `Tunnel::udp_socket()` directly — sends a raw DNS query
//! (hand-crafted binary packet) to Cloudflare's 1.1.1.1 and reads the response.
//! This is the lowest level of mixnet usage: raw datagrams, no libraries.
//!
//! Compares a clearnet UDP socket (tokio) with a mixnet UDP socket (smolmix).
//!
//! Run with:
//!   cargo run -p smolmix --example udp
//!   cargo run -p smolmix --example udp -- --ipr <IPR_ADDRESS>

use std::net::SocketAddr;

use smolmix::{Recipient, Tunnel};
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const DNS_SERVER: &str = "1.1.1.1:53";

/// Build a minimal DNS query for example.com A record.
///
/// This is a hand-crafted DNS wire-format packet — no dependencies needed.
fn dns_query() -> Vec<u8> {
    let mut pkt = Vec::new();
    // Header: ID=0x1234, QR=0, OPCODE=0, RD=1, QDCOUNT=1
    pkt.extend_from_slice(&[0x12, 0x34]); // ID
    pkt.extend_from_slice(&[0x01, 0x00]); // Flags: RD=1
    pkt.extend_from_slice(&[0x00, 0x01]); // QDCOUNT=1
    pkt.extend_from_slice(&[0x00, 0x00]); // ANCOUNT=0
    pkt.extend_from_slice(&[0x00, 0x00]); // NSCOUNT=0
    pkt.extend_from_slice(&[0x00, 0x00]); // ARCOUNT=0
                                          // Question: example.com, Type A, Class IN
    pkt.extend_from_slice(&[7]); // length of "example"
    pkt.extend_from_slice(b"example");
    pkt.extend_from_slice(&[3]); // length of "com"
    pkt.extend_from_slice(b"com");
    pkt.push(0); // root label
    pkt.extend_from_slice(&[0x00, 0x01]); // Type A
    pkt.extend_from_slice(&[0x00, 0x01]); // Class IN
    pkt
}

/// Parse the answer section of a DNS response to extract A record IPs.
fn parse_dns_response(buf: &[u8]) -> Vec<std::net::Ipv4Addr> {
    let mut ips = Vec::new();
    if buf.len() < 12 {
        return ips;
    }
    let ancount = u16::from_be_bytes([buf[6], buf[7]]) as usize;

    // Skip header (12 bytes) and question section
    let mut pos = 12;
    // Skip question: labels + null + QTYPE(2) + QCLASS(2)
    while pos < buf.len() && buf[pos] != 0 {
        let len = buf[pos] as usize;
        pos += 1 + len;
    }
    pos += 1 + 4; // null byte + QTYPE + QCLASS

    // Parse answer records
    for _ in 0..ancount {
        if pos + 12 > buf.len() {
            break;
        }
        // Skip name (may be compressed pointer)
        if buf[pos] & 0xC0 == 0xC0 {
            pos += 2; // compressed pointer
        } else {
            while pos < buf.len() && buf[pos] != 0 {
                let len = buf[pos] as usize;
                pos += 1 + len;
            }
            pos += 1;
        }
        let rtype = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
        let rdlength = u16::from_be_bytes([buf[pos + 8], buf[pos + 9]]) as usize;
        pos += 10; // TYPE(2) + CLASS(2) + TTL(4) + RDLENGTH(2)

        if rtype == 1 && rdlength == 4 && pos + 4 <= buf.len() {
            ips.push(std::net::Ipv4Addr::new(
                buf[pos],
                buf[pos + 1],
                buf[pos + 2],
                buf[pos + 3],
            ));
        }
        pos += rdlength;
    }
    ips
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    smolmix::init_logging();

    let dns_addr: SocketAddr = DNS_SERVER.parse()?;
    let query = dns_query();

    // --- Clearnet baseline via tokio ---
    info!("Sending DNS query via clearnet to {dns_addr}...");
    let clearnet_start = tokio::time::Instant::now();
    let clearnet_udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    clearnet_udp.send_to(&query, dns_addr).await?;
    let mut clearnet_buf = [0u8; 512];
    let (clearnet_len, _) = clearnet_udp.recv_from(&mut clearnet_buf).await?;
    let clearnet_duration = clearnet_start.elapsed();
    let clearnet_ips = parse_dns_response(&clearnet_buf[..clearnet_len]);
    info!(
        "Clearnet: {:?} ({} bytes, {:?})",
        clearnet_ips, clearnet_len, clearnet_duration
    );

    // --- Mixnet via smolmix ---
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

    info!("Sending DNS query via mixnet to {dns_addr}...");
    let mixnet_start = tokio::time::Instant::now();
    let mixnet_udp = tunnel.udp_socket().await?;
    mixnet_udp.send_to(&query, dns_addr).await?;
    let mut mixnet_buf = [0u8; 512];
    let (mixnet_len, _) = mixnet_udp.recv_from(&mut mixnet_buf).await?;
    let mixnet_duration = mixnet_start.elapsed();
    let mixnet_ips = parse_dns_response(&mixnet_buf[..mixnet_len]);

    // --- Compare ---
    info!("=== Results ===");
    info!(
        "Clearnet: {:?} ({} bytes, {:?})",
        clearnet_ips, clearnet_len, clearnet_duration
    );
    info!(
        "Mixnet:   {:?} ({} bytes, {:?})",
        mixnet_ips, mixnet_len, mixnet_duration
    );

    let ips_match = !mixnet_ips.is_empty() && mixnet_ips.iter().all(|ip| clearnet_ips.contains(ip));
    info!("IPs match: {ips_match}");

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
