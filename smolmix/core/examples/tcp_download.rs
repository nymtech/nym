// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Sequential HTTPS downloads of varying sizes through the Nym mixnet.
//!
//! Resolves `httpbin.org` over mixnet UDP DNS, then issues four GET
//! `/bytes/{N}` requests over a single keep-alive HTTP/1.1 connection.
//! All traffic, including the DNS lookup, stays inside the tunnel.
//!
//! Compare with `tcp.rs` for a single-request version with a clearnet
//! baseline.
//!
//! ```text
//! hyper (HTTP/1.1 client, keep-alive)
//!   └─ tokio-rustls (TLS encryption)
//!        └─ smolmix::TcpStream (TCP over mixnet)
//!             └─ smoltcp (userspace TCP/IP)
//!                  └─ Nym mixnet → IPR exit gateway → internet
//! ```
//!
//! ```sh
//! cargo run -p smolmix --example tcp_download
//! cargo run -p smolmix --example tcp_download -- --ipr <IPR_ADDRESS>
//! ```

use std::net::Ipv4Addr;
use std::sync::Arc;

use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use smolmix::Tunnel;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const HOST: &str = "httpbin.org";

/// Sizes (in bytes) to download sequentially over one connection.
const SIZES: &[usize] = &[100, 1_000, 10_000, 25_000];

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

    let mut builder = Tunnel::builder();
    if let Some(addr) = ipr_addr {
        builder = builder.ipr_address(addr.parse().expect("invalid IPR address"));
    }
    let tunnel = builder.build().await?;
    println!(
        "Tunnel ready, allocated IP: {}",
        tunnel.allocated_ips().ipv4
    );

    // hickory-proto sends the DNS query over the tunnel directly, so the
    // lookup never touches the system resolver.
    let ip = resolve_dns(&tunnel, HOST).await?;
    println!("Resolved {HOST} → {ip} (via mixnet DNS)");

    println!("Connecting to {HOST}:443...");
    let tcp = tunnel.tcp_connect((ip, 443).into()).await?;
    println!("TCP connected to {ip}:443 via mixnet");

    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config));
    let domain = ServerName::try_from(HOST)?.to_owned();
    let tls = connector.connect(domain, tcp).await?;
    println!("TLS established with {HOST}");

    let io = TokioIo::new(tls);
    let (mut sender, conn) = http1::handshake(io).await?;
    tokio::spawn(conn);

    let total = SIZES.len();
    println!("Sending {total} requests over one connection...");
    let overall = std::time::Instant::now();
    let mut total_bytes = 0usize;

    for (i, &size) in SIZES.iter().enumerate() {
        let seq = i + 1;
        let start = std::time::Instant::now();

        let req = hyper::Request::get(format!("/bytes/{size}"))
            .header("Host", HOST)
            .body(Empty::<Bytes>::new())?;

        let spinner = tokio::spawn(async move {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;
            loop {
                eprint!(
                    "\r  [{seq}/{total}] GET /bytes/{size:<5} {}",
                    frames[i % frames.len()]
                );
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        });

        let resp = sender.send_request(req).await?;
        let status = resp.status();
        let body = resp.into_body().collect().await?.to_bytes();
        let elapsed = start.elapsed();
        spinner.abort();
        let _ = spinner.await;

        let speed = body.len() as f64 / elapsed.as_secs_f64();
        eprintln!(
            "\r  [{seq}/{total}] GET /bytes/{size:<5} → {status} {} in {elapsed:.1?} ({}/s)   ",
            format_bytes(body.len() as u64),
            format_bytes(speed as u64),
        );
        total_bytes += body.len();
    }

    let elapsed = overall.elapsed();
    println!(
        "Done: {} across {total} requests in {elapsed:.1?}",
        format_bytes(total_bytes as u64),
    );

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

fn format_bytes(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1} MB", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1} KB", n as f64 / 1_000.0)
    } else {
        format!("{n} B")
    }
}
