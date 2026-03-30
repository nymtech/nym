//! Raw TCP connection through the Nym mixnet.
//!
//! Demonstrates using `Tunnel::tcp_connect()` directly — no DNS, no TLS,
//! just a raw `TcpStream` (AsyncRead + AsyncWrite) over the mixnet. Sends an
//! HTTP/1.1 request by hand to show the raw bytes going through.
//!
//! Compares a clearnet TCP connection (tokio) with a mixnet TCP connection
//! (smolmix) to the same IP address.
//!
//! Run with:
//!   cargo run -p smolmix --example tcp
//!   cargo run -p smolmix --example tcp -- --ipr <IPR_ADDRESS>

use std::net::SocketAddr;

use smolmix::{Recipient, Tunnel};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// httpbin.org resolves to this — hardcoded to avoid DNS dependency in this example
const TARGET: &str = "1.1.1.1:80";
const HTTP_REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nConnection: close\r\n\r\n";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    smolmix::init_logging();

    let addr: SocketAddr = TARGET.parse()?;

    // --- Clearnet baseline via tokio ---
    info!("Connecting via clearnet to {addr}...");
    let clearnet_start = tokio::time::Instant::now();
    let mut clearnet_tcp = tokio::net::TcpStream::connect(addr).await?;
    clearnet_tcp.write_all(HTTP_REQUEST).await?;
    let mut clearnet_buf = Vec::new();
    clearnet_tcp.read_to_end(&mut clearnet_buf).await?;
    let clearnet_duration = clearnet_start.elapsed();
    let clearnet_status = String::from_utf8_lossy(&clearnet_buf)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    info!(
        "Clearnet: \"{clearnet_status}\" ({} bytes, {:?})",
        clearnet_buf.len(),
        clearnet_duration
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

    info!("Connecting via mixnet to {addr}...");
    let mixnet_start = tokio::time::Instant::now();
    let mut mixnet_tcp = tunnel.tcp_connect(addr).await?;
    mixnet_tcp.write_all(HTTP_REQUEST).await?;
    let mut mixnet_buf = Vec::new();
    mixnet_tcp.read_to_end(&mut mixnet_buf).await?;
    let mixnet_duration = mixnet_start.elapsed();
    let mixnet_status = String::from_utf8_lossy(&mixnet_buf)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();

    // --- Compare ---
    info!("=== Results ===");
    info!(
        "Clearnet: \"{clearnet_status}\" ({} bytes, {:?})",
        clearnet_buf.len(),
        clearnet_duration
    );
    info!(
        "Mixnet:   \"{mixnet_status}\" ({} bytes, {:?})",
        mixnet_buf.len(),
        mixnet_duration
    );

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
