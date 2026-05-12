// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Burst echo + idle survival over the Nym mixnet via WebSocket.
//!
//! Sends multiple text messages of varying sizes, then probes
//! the connection after increasing idle gaps. A clearnet baseline
//! runs first to isolate server-side vs mixnet-side idle timeouts.
//!
//! ```text
//! tokio-tungstenite (WebSocket framing)
//!   └─ tokio-rustls (TLS encryption)
//!        ├─ tokio::net::TcpStream  (clearnet baseline)
//!        └─ smolmix::TcpStream     (mixnet)
//! ```
//!
//! ## What this demonstrates
//!
//! - Connection reuse: a single WebSocket handles both burst traffic and
//!   idle probes, since the setup cost is paid once
//! - Per-message latency vs payload size over the mixnet
//! - Idle timeout characterisation: clearnet baseline isolates whether
//!   connection drops are server-side or mixnet-side
//!
//! Compare with `websocket.rs` which sends a single echo with clearnet comparison.
//!
//! ```sh
//! cargo run -p smolmix --example websocket_multi
//! cargo run -p smolmix --example websocket_multi -- --ipr <IPR_ADDRESS>
//! ```

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rustls::pki_types::ServerName;
use smolmix::Tunnel;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::{self, Message};
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const WS_HOST: &str = "ws.postman-echo.com";
const WS_PATH: &str = "/raw";
const RECV_TIMEOUT: Duration = Duration::from_secs(30);
const CLEARNET_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_GAPS: &[u64] = &[1, 2, 3, 5, 10];

fn tls_connector() -> tokio_rustls::TlsConnector {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

fn burst_messages() -> Vec<Message> {
    vec![
        Message::Text("hello!".into()),
        Message::Text("x".repeat(100)),
        Message::Text("x".repeat(1024)),
        Message::Text("x".repeat(10240)),
        Message::Text("final probe".into()),
    ]
}

/// Format a byte count for display (B, KB).
fn fmt_size(n: usize) -> String {
    if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

/// Format a duration as seconds with one decimal place.
fn fmt_secs(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f64())
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Resolve hostname via clearnet DNS
    let addr = tokio::net::lookup_host(format!("{WS_HOST}:443"))
        .await?
        .next()
        .ok_or("DNS resolution failed")?;
    info!("Resolved {WS_HOST} -> {addr}");

    let connector = tls_connector();
    let domain = ServerName::try_from(WS_HOST)?.to_owned();

    // Clearnet idle baseline
    info!("Clearnet idle baseline...");
    let clearnet_tcp = tokio::net::TcpStream::connect(addr).await?;
    let clearnet_tls = connector.connect(domain.clone(), clearnet_tcp).await?;
    let (mut clearnet_ws, _) =
        tokio_tungstenite::client_async(format!("wss://{WS_HOST}{WS_PATH}"), clearnet_tls).await?;

    // Verify the connection works
    clearnet_ws.send(Message::Text("ping".into())).await?;
    let _ = clearnet_ws.next().await.ok_or("no clearnet reply")??;

    let mut clearnet_idle: Vec<(u64, Option<Duration>)> = Vec::new();

    for gap in IDLE_GAPS {
        tokio::time::sleep(Duration::from_secs(*gap)).await;

        let probe = format!("clearnet probe after {gap}s");
        let start = Instant::now();

        let result = tokio::time::timeout(CLEARNET_TIMEOUT, async {
            clearnet_ws.send(Message::Text(probe.clone())).await?;
            clearnet_ws
                .next()
                .await
                .ok_or(tungstenite::Error::ConnectionClosed)?
        })
        .await;

        match result {
            Ok(Ok(reply)) => {
                let rtt = start.elapsed();
                let reply_text = reply.into_text().unwrap_or_default();
                let matched = reply_text == probe;
                info!(
                    "  {gap}s idle -> {}  {}",
                    fmt_secs(rtt),
                    if matched { "ok" } else { "MISMATCH" }
                );
                clearnet_idle.push((*gap, Some(rtt)));
            }
            _ => {
                info!("  {gap}s idle -> dropped");
                clearnet_idle.push((*gap, None));
                break;
            }
        }
    }
    let _ = clearnet_ws.close(None).await;

    // Mixnet path
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
    info!("Allocated IP: {}", tunnel.allocated_ips().ipv4);

    // TCP + TLS + WebSocket handshake through the mixnet
    let setup_start = Instant::now();

    info!("TCP connecting via mixnet...");
    let mixnet_tcp = tunnel.tcp_connect(addr).await?;
    let tcp_elapsed = setup_start.elapsed();
    info!("TCP connected ({})", fmt_secs(tcp_elapsed));

    info!("TLS handshake...");
    let mixnet_tls = connector.connect(domain, mixnet_tcp).await?;
    let tls_elapsed = setup_start.elapsed();
    info!("TLS established ({})", fmt_secs(tls_elapsed));

    info!("WebSocket upgrade...");
    let (mut ws, _) =
        tokio_tungstenite::client_async(format!("wss://{WS_HOST}{WS_PATH}"), mixnet_tls).await?;
    let setup_elapsed = setup_start.elapsed();
    info!("Setup complete ({})", fmt_secs(setup_elapsed));

    // Burst - send messages of varying sizes
    info!("Starting burst phase...");
    let messages = burst_messages();
    let msg_count = messages.len();
    let mut total_rtt = Duration::ZERO;

    for (i, msg) in messages.iter().enumerate() {
        let Message::Text(expected) = msg else {
            unreachable!("burst messages are all text");
        };
        let size = expected.len();

        let start = Instant::now();
        let reply = tokio::time::timeout(RECV_TIMEOUT, async {
            ws.send(msg.clone()).await?;
            ws.next()
                .await
                .ok_or(tungstenite::Error::ConnectionClosed)?
        })
        .await
        .map_err(|_| "round-trip timeout")??;

        let rtt = start.elapsed();
        total_rtt += rtt;
        let matched = reply.into_text()? == *expected;

        info!(
            "  [{}/{}] text {:>8}  {}  {}",
            i + 1,
            msg_count,
            fmt_size(size),
            fmt_secs(rtt),
            if matched { "ok" } else { "MISMATCH" }
        );
    }

    let avg_rtt = total_rtt / msg_count as u32;
    info!("  avg RTT: {}", fmt_secs(avg_rtt));

    // Idle survival: wait, then probe
    info!("Starting idle survival phase...");
    let mut mixnet_idle: Vec<(u64, Option<Duration>)> = Vec::new();

    for gap in IDLE_GAPS {
        info!("  waiting {gap}s...");
        tokio::time::sleep(Duration::from_secs(*gap)).await;

        let probe = format!("probe after {gap}s");
        let start = Instant::now();

        let result = tokio::time::timeout(RECV_TIMEOUT, async {
            ws.send(Message::Text(probe.clone())).await?;
            ws.next()
                .await
                .ok_or(tungstenite::Error::ConnectionClosed)?
        })
        .await;

        match result {
            Ok(Ok(reply)) => {
                let rtt = start.elapsed();
                let reply_text = reply.into_text().unwrap_or_default();
                let matched = reply_text == probe;
                info!(
                    "  {gap}s idle -> {}  {}",
                    fmt_secs(rtt),
                    if matched { "ok" } else { "MISMATCH" }
                );
                mixnet_idle.push((*gap, Some(rtt)));
            }
            _ => {
                info!("  {gap}s idle -> dropped");
                mixnet_idle.push((*gap, None));
                break;
            }
        }
    }

    // Summary
    let tls_only = tls_elapsed - tcp_elapsed;
    let ws_only = setup_elapsed - tls_elapsed;

    info!("");
    info!(
        "Setup {}  (TCP {} + TLS {} + WS {})",
        fmt_secs(setup_elapsed),
        fmt_secs(tcp_elapsed),
        fmt_secs(tls_only),
        fmt_secs(ws_only),
    );
    info!(
        "Burst avg RTT: {} ({msg_count} messages)",
        fmt_secs(avg_rtt)
    );
    info!("");
    info!("Idle survival (clearnet)");
    for (gap, result) in &clearnet_idle {
        match result {
            Some(rtt) => info!("{gap}s  idle -> {}  ok", fmt_secs(*rtt)),
            None => info!("{gap}s  idle -> dropped"),
        }
    }
    info!("Idle survival (mixnet)");
    for (gap, result) in &mixnet_idle {
        match result {
            Some(rtt) => info!("{gap}s  idle -> {}  ok", fmt_secs(*rtt)),
            None => info!("{gap}s  idle -> dropped"),
        }
    }

    let _ = ws.close(None).await;
    tunnel.shutdown().await;
    Ok(())
}
