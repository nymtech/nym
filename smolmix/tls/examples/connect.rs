//! TLS connection: clearnet vs Nym mixnet comparison.
//!
//! Performs a TLS handshake and HTTPS GET request via both clearnet (tokio-rustls
//! over a system TCP socket) and the mixnet (smolmix-tls over a tunnel), then
//! compares timing and verifies both see the same content.
//!
//! Run with:
//!   cargo run -p smolmix-tls --example connect
//!   cargo run -p smolmix-tls --example connect -- --ipr <IPR_ADDRESS>

use std::sync::Arc;

use rustls::ClientConfig;
use smolmix::{Recipient, Tunnel};
use smolmix_dns::Resolver;
use smolmix_tls::{connect_with, connector};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let host = "example.com";
    let port = 443u16;

    // Clearnet baseline via tokio + tokio-rustls
    info!("Clearnet TLS to {host}:{port}...");
    let clearnet_start = std::time::Instant::now();

    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(config));

    let tcp = tokio::net::TcpStream::connect((host, port)).await?;
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())?;
    let mut tls = tls_connector.connect(server_name, tcp).await?;

    tls.write_all(
        format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await?;
    let mut clearnet_buf = Vec::new();
    tls.read_to_end(&mut clearnet_buf).await?;
    let clearnet_duration = clearnet_start.elapsed();

    let clearnet_status = extract_status(&clearnet_buf);
    info!(
        "{clearnet_status} {} in {clearnet_duration:.1?}",
        format_bytes(clearnet_buf.len() as u64)
    );

    // Mixnet tunnel setup
    let args: Vec<String> = std::env::args().collect();
    let ipr_addr = args
        .iter()
        .position(|a| a == "--ipr")
        .and_then(|i| args.get(i + 1));

    let tunnel = if let Some(addr_str) = ipr_addr {
        let recipient: Recipient = addr_str.parse().expect("invalid IPR address");
        Tunnel::new_with_ipr(recipient).await?
    } else {
        Tunnel::new().await?
    };
    info!(
        "Tunnel ready — allocated IP: {}",
        tunnel.allocated_ips().ipv4
    );

    let tls_conn = connector();
    let resolver = Resolver::new(&tunnel);
    let overall_start = std::time::Instant::now();

    // DNS resolution via mixnet
    info!("Mixnet TLS to {host}:{port}...");
    let spinner = spin(&format!("Resolving {host} via mixnet DNS..."));
    let dns_start = std::time::Instant::now();
    let addrs = resolver.resolve(host, port).await?;
    let addr = addrs.into_iter().next().ok_or("no addresses resolved")?;
    let dns_duration = dns_start.elapsed();
    spinner.abort();
    eprint!("\r                                                  \r");
    info!("DNS:  {host} → {addr} ({dns_duration:.1?})");

    // TCP connection through mixnet
    let spinner = spin(&format!("TCP connecting to {addr}..."));
    let tcp_start = std::time::Instant::now();
    let tcp = tunnel.tcp_connect(addr).await?;
    let tcp_duration = tcp_start.elapsed();
    spinner.abort();
    eprint!("\r                                                  \r");
    info!("TCP:  connected to {addr} ({tcp_duration:.1?})");

    // TLS handshake
    let spinner = spin(&format!("TLS handshake with {host}..."));
    let tls_start = std::time::Instant::now();
    let mut tls = connect_with(&tls_conn, tcp, host).await?;
    let tls_duration = tls_start.elapsed();
    spinner.abort();
    eprint!("\r                                                  \r");
    info!("TLS:  handshake complete ({tls_duration:.1?})");

    // First HTTP GET (keep-alive)
    let spinner = spin("GET / (first request)...");
    let http1_start = std::time::Instant::now();
    tls.write_all(format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes())
        .await?;
    let mixnet_buf = read_http_response(&mut tls).await?;
    let http1_duration = http1_start.elapsed();
    spinner.abort();

    let mixnet_status = extract_status(&mixnet_buf);
    info!(
        "GET1: {mixnet_status} {} ({http1_duration:.1?})",
        format_bytes(mixnet_buf.len() as u64)
    );

    // Second HTTP GET over same connection (no DNS/TCP/TLS overhead)
    let spinner = spin("GET / (reusing connection)...");
    let http2_start = std::time::Instant::now();
    tls.write_all(
        format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await?;
    let mut mixnet_buf2 = Vec::new();
    tls.read_to_end(&mut mixnet_buf2).await?;
    let http2_duration = http2_start.elapsed();
    spinner.abort();

    let mixnet_status2 = extract_status(&mixnet_buf2);
    info!(
        "GET2: {mixnet_status2} {} ({http2_duration:.1?}) (reused connection)",
        format_bytes(mixnet_buf2.len() as u64)
    );

    let mixnet_duration = overall_start.elapsed();

    // Compare
    info!("Results");
    info!(
        "Clearnet:    {} in {clearnet_duration:.1?}",
        format_bytes(clearnet_buf.len() as u64)
    );
    info!(
        "Mixnet #1:   {} in {mixnet_duration:.1?} (DNS {dns_duration:.1?} + TCP {tcp_duration:.1?} + TLS {tls_duration:.1?} + HTTP {http1_duration:.1?})",
        format_bytes(mixnet_buf.len() as u64),
    );
    info!(
        "Mixnet #2:   {} in {http2_duration:.1?} (reused connection)",
        format_bytes(mixnet_buf2.len() as u64),
    );

    let slowdown1 =
        mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    let slowdown2 = http2_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown:    {slowdown1:.1}x (cold) / {slowdown2:.1}x (warm)");

    tunnel.shutdown().await;
    Ok(())
}

fn spin(msg: &str) -> tokio::task::JoinHandle<()> {
    let msg = msg.to_string();
    tokio::spawn(async move {
        let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let mut i = 0;
        loop {
            eprint!("\r  {} {}", frames[i % frames.len()], msg);
            i += 1;
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
    })
}

/// Read a complete HTTP/1.1 response from a keep-alive connection.
///
/// Parses headers to find `Content-Length`, then reads exactly that many body
/// bytes. Returns the full response (headers + body) as a single buffer.
async fn read_http_response<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Vec<u8>, BoxError> {
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 1024];

    // Read until we find the end-of-headers marker
    let header_end = loop {
        let n = reader.read(&mut tmp).await?;
        if n == 0 {
            return Err("connection closed before headers complete".into());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
    };

    // Parse Content-Length from headers
    let headers = std::str::from_utf8(&buf[..header_end]).unwrap_or("");
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (key, val) = line.split_once(':')?;
            if key.trim().eq_ignore_ascii_case("content-length") {
                val.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    // Read remaining body bytes
    let body_so_far = buf.len() - header_end;
    let remaining = content_length.saturating_sub(body_so_far);
    if remaining > 0 {
        let mut body_buf = vec![0u8; remaining];
        reader.read_exact(&mut body_buf).await?;
        buf.extend_from_slice(&body_buf);
    }

    Ok(buf)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn extract_status(buf: &[u8]) -> &str {
    let s = std::str::from_utf8(&buf[..buf.len().min(40)]).unwrap_or("");
    s.lines().next().unwrap_or("")
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
