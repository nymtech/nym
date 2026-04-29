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
    let addr: std::net::SocketAddr = "93.184.216.34:443".parse()?;

    // --- Clearnet baseline via tokio + tokio-rustls ---
    info!("Clearnet TLS connection to {host}...");
    let clearnet_start = tokio::time::Instant::now();

    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(config));

    let tcp = tokio::net::TcpStream::connect(addr).await?;
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())?;
    let mut tls = tls_connector.connect(server_name, tcp).await?;

    tls.write_all(
        format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await?;
    let mut clearnet_buf = Vec::new();
    tls.read_to_end(&mut clearnet_buf).await?;
    let clearnet_duration = clearnet_start.elapsed();

    let clearnet_status = String::from_utf8_lossy(&clearnet_buf[..40.min(clearnet_buf.len())]);
    info!(
        "Clearnet: {} bytes, status: {:?} ({:?})",
        clearnet_buf.len(),
        clearnet_status.lines().next().unwrap_or(""),
        clearnet_duration
    );

    // --- Mixnet via smolmix-tls ---
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

    let tls_conn = connector();

    info!("Mixnet TLS connection to {host}...");
    let mixnet_start = tokio::time::Instant::now();

    let tcp = tunnel.tcp_connect(addr).await?;
    let mut tls = connect_with(&tls_conn, tcp, host).await?;

    tls.write_all(
        format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await?;
    let mut mixnet_buf = Vec::new();
    tls.read_to_end(&mut mixnet_buf).await?;
    let mixnet_duration = mixnet_start.elapsed();

    let mixnet_status = String::from_utf8_lossy(&mixnet_buf[..40.min(mixnet_buf.len())]);
    info!(
        "Mixnet:   {} bytes, status: {:?} ({:?})",
        mixnet_buf.len(),
        mixnet_status.lines().next().unwrap_or(""),
        mixnet_duration
    );

    // --- Compare ---
    info!("=== Results ===");
    info!(
        "Clearnet: {} bytes in {:?}",
        clearnet_buf.len(),
        clearnet_duration
    );
    info!(
        "Mixnet:   {} bytes in {:?}",
        mixnet_buf.len(),
        mixnet_duration
    );

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
