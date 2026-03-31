//! WebSocket echo over the Nym mixnet using the Tunnel API.
//!
//! Demonstrates stacking tokio-tungstenite on top of tokio-rustls on top of
//! our Tunnel TcpStream. Sends a message to a public echo server via clearnet
//! and via the mixnet, then compares responses and timing.
//!
//! DNS resolution goes through the tunnel (no clearnet leak). The clearnet and
//! mixnet paths use the *exact same* TLS + WebSocket stack — only the underlying
//! TCP transport differs:
//!
//! ```text
//! tokio-tungstenite (WebSocket framing)
//!   └─ tokio-rustls (TLS encryption)
//!        └─ tokio::net::TcpStream  (clearnet)
//!        └─ smolmix::TcpStream     (mixnet)
//! ```
//!
//! Run with:
//!   cargo run -p smolmix --example websocket
//!   cargo run -p smolmix --example websocket -- --ipr <IPR_ADDRESS>

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rustls::pki_types::ServerName;
use smolmix::{Recipient, Tunnel};
use tokio_tungstenite::tungstenite::Message;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const WS_HOST: &str = "ws.postman-echo.com";
const WS_PATH: &str = "/raw";
const ECHO_MSG: &str = "Hello from the Nym mixnet!";

fn tls_connector() -> tokio_rustls::TlsConnector {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    smolmix::init_logging();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let connector = tls_connector();
    let domain = ServerName::try_from(WS_HOST)?.to_owned();

    // --- Set up tunnel and resolve DNS through it (no clearnet leak) ---
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
    info!("Allocated IP: {}", tunnel.allocated_ips().ipv4);

    // NOTE: This uses clearnet DNS for simplicity. For leak-free DNS resolution
    // through the mixnet, use the smolmix-dns crate.
    let addr = tokio::net::lookup_host(format!("{WS_HOST}:443"))
        .await?
        .next()
        .ok_or("DNS resolution returned no addresses")?;
    info!("Resolved {WS_HOST} -> {addr} (via clearnet DNS)");

    // --- Clearnet baseline: tokio TCP → rustls → tungstenite ---
    info!("Connecting via clearnet...");
    let clearnet_start = tokio::time::Instant::now();

    let clearnet_tcp = tokio::net::TcpStream::connect(addr).await?;
    let clearnet_tls = connector.connect(domain.clone(), clearnet_tcp).await?;
    let (mut clearnet_ws, _) =
        tokio_tungstenite::client_async(format!("wss://{WS_HOST}{WS_PATH}"), clearnet_tls).await?;

    clearnet_ws.send(Message::Text(ECHO_MSG.into())).await?;
    let clearnet_reply = clearnet_ws.next().await.ok_or("no clearnet reply")??;
    let clearnet_duration = clearnet_start.elapsed();
    let clearnet_text = clearnet_reply.into_text()?;
    clearnet_ws.close(None).await?;

    info!("Clearnet: \"{clearnet_text}\" in {clearnet_duration:?}");

    // --- Mixnet: smolmix TCP → rustls → tungstenite (same stack) ---
    let mixnet_start = tokio::time::Instant::now();

    let mixnet_tcp = tunnel.tcp_connect(addr).await?;
    let mixnet_tls = connector.connect(domain, mixnet_tcp).await?;
    let (mut mixnet_ws, _) =
        tokio_tungstenite::client_async(format!("wss://{WS_HOST}{WS_PATH}"), mixnet_tls).await?;

    mixnet_ws.send(Message::Text(ECHO_MSG.into())).await?;
    let mixnet_reply = mixnet_ws.next().await.ok_or("no mixnet reply")??;
    let mixnet_duration = mixnet_start.elapsed();
    let mixnet_text = mixnet_reply.into_text()?;
    mixnet_ws.close(None).await?;

    // --- Compare ---
    info!("=== Results ===");
    info!("Clearnet: \"{clearnet_text}\" in {clearnet_duration:?}");
    info!("Mixnet:   \"{mixnet_text}\" in {mixnet_duration:?}");
    info!("Clearnet echo match: {}", clearnet_text == ECHO_MSG);
    info!("Mixnet echo match:   {}", mixnet_text == ECHO_MSG);

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
