// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! HTTPS request through the Nym mixnet.
//!
//! Fetches Cloudflare's `/cdn-cgi/trace` over clearnet (reqwest) and through
//! the mixnet (hyper over tokio-rustls over smolmix), then compares the exit
//! IPs. The mixnet path should show a different IP, since traffic exits through
//! an IPR (Internet Packet Router) gateway, not your machine.
//!
//! ```text
//! hyper (HTTP/1.1 client)
//!   └─ tokio-rustls (TLS encryption)
//!        └─ smolmix::TcpStream (TCP over mixnet)
//!             └─ smoltcp (userspace TCP/IP)
//!                  └─ Nym mixnet → IPR exit gateway → internet
//! ```
//!
//! ## What this demonstrates
//!
//! - Creating a [`Tunnel`] and connecting TCP through the mixnet
//! - Layering TLS ([`tokio_rustls`]) on a [`smolmix::TcpStream`]: it
//!   implements `AsyncRead + AsyncWrite`, so standard crates work unchanged
//! - Using [`hyper`]'s HTTP/1.1 client over a custom transport via
//!   [`TokioIo`](hyper_util::rt::TokioIo)
//! - The exit IP differs from clearnet. The remote server sees the IPR
//!   gateway's IP, not yours
//!
//! ```sh
//! cargo run -p smolmix --example tcp
//! cargo run -p smolmix --example tcp -- --ipr <IPR_ADDRESS>
//! ```

use std::sync::Arc;

use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::Request;
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use smolmix::Tunnel;
use tokio_rustls::TlsConnector;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const HOST: &str = "cloudflare.com";
const PATH: &str = "/cdn-cgi/trace";

fn tls_connector() -> TlsConnector {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Clearnet baseline via reqwest
    info!("Fetching via clearnet...");
    let clearnet_start = tokio::time::Instant::now();
    let clearnet_resp = reqwest::get(format!("https://{HOST}{PATH}")).await?;
    let clearnet_status = clearnet_resp.status();
    let clearnet_body = clearnet_resp.text().await?;
    let clearnet_duration = clearnet_start.elapsed();
    info!("Clearnet: {} in {:?}", clearnet_status, clearnet_duration);

    // Mixnet path
    // Create a tunnel, then stack the same TLS + HTTP layers on top.
    // The only difference: smolmix::TcpStream instead of tokio::net::TcpStream.

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

    // TCP + TLS + HTTP handshakes through the mixnet.
    // tcp_connect() returns a TcpStream that implements AsyncRead + AsyncWrite.
    // tokio-rustls accepts it directly, no adapters or trait shims needed.
    // TokioIo then bridges hyper's I/O traits with tokio's.
    let setup_start = tokio::time::Instant::now();

    info!("TCP connecting to 1.1.1.1:443 via mixnet...");
    let tcp = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;
    info!("TCP connected ({:?})", setup_start.elapsed());

    info!("TLS handshake...");
    let connector = tls_connector();
    let domain = ServerName::try_from(HOST)?.to_owned();
    let tls = connector.connect(domain, tcp).await?;
    info!("TLS established ({:?})", setup_start.elapsed());

    info!("HTTP/1.1 handshake...");
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(tls)).await?;
    tokio::spawn(conn);

    let setup_duration = setup_start.elapsed();
    info!("Setup complete ({:?})", setup_duration);

    // Send request and read response.
    // From here the code is identical to any hyper client. The mixnet
    // transport is invisible to higher layers.
    let request_start = tokio::time::Instant::now();

    info!("Sending GET {PATH}...");
    let req = Request::get(PATH)
        .header("Host", HOST)
        .body(http_body_util::Empty::<Bytes>::new())?;
    let resp = sender.send_request(req).await?;
    let mixnet_status = resp.status();
    let body_bytes = resp.into_body().collect().await?.to_bytes();
    let mixnet_body = String::from_utf8_lossy(&body_bytes);

    let request_duration = request_start.elapsed();
    info!(
        "Response: {} ({} bytes, {:?})",
        mixnet_status,
        body_bytes.len(),
        request_duration
    );

    // Results
    let clearnet_ip = clearnet_body.lines().find(|l| l.starts_with("ip="));
    let mixnet_ip = mixnet_body.lines().find(|l| l.starts_with("ip="));

    info!("Clearnet: {} in {:?}", clearnet_status, clearnet_duration);
    info!(
        "Mixnet: {} (setup {:?} + request {:?} = {:?})",
        mixnet_status,
        setup_duration,
        request_duration,
        setup_duration + request_duration
    );
    info!("Clearnet IP: {}", clearnet_ip.unwrap_or("?"));
    info!("Mixnet IP: {}", mixnet_ip.unwrap_or("?"));

    let total = setup_duration + request_duration;
    let slowdown = total.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!(
        "Slowdown: {slowdown:.1}x (setup: {:.1}x, request: {:.1}x)",
        setup_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64,
        request_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64
    );

    tunnel.shutdown().await;
    Ok(())
}
