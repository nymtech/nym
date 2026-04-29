//! HTTPS GET: clearnet vs Nym mixnet.
//!
//! Fetches Cloudflare's `/cdn-cgi/trace` endpoint via clearnet and through the
//! mixnet. The same `https_get` function handles both — the only difference is
//! the TCP stream passed in:
//!
//! ```text
//! hyper (HTTP/1.1 framing)
//!   └─ tokio-rustls (TLS)
//!        └─ tokio::net::TcpStream  (clearnet)
//!        └─ smolmix::TcpStream     (mixnet)
//! ```
//!
//! Run with:
//!   cargo run -p smolmix --example https
//!   cargo run -p smolmix --example https -- --ipr <IPR_ADDRESS>

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use smolmix::{Recipient, Tunnel};
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const HOST: &str = "cloudflare.com";
const ADDR: &str = "1.1.1.1:443"; // cloudflare.com — hardcoded to skip DNS
const PATH: &str = "/cdn-cgi/trace";

fn tls_connector() -> tokio_rustls::TlsConnector {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

/// HTTPS GET over any TCP stream — works identically with clearnet and mixnet.
async fn https_get(
    tcp: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
    tls: &tokio_rustls::TlsConnector,
    host: &str,
    path: &str,
) -> Result<(hyper::StatusCode, String), BoxError> {
    let domain = ServerName::try_from(host)?.to_owned();
    let tls_stream = tls.connect(domain, tcp).await?;
    let (mut sender, conn) = http1::handshake(TokioIo::new(tls_stream)).await?;
    tokio::spawn(conn);

    let req = hyper::Request::get(path)
        .header("host", host)
        .body(Empty::<Bytes>::new())?;
    let resp = sender.send_request(req).await?;
    let status = resp.status();
    let body = resp.into_body().collect().await?.to_bytes();
    Ok((status, String::from_utf8_lossy(&body).to_string()))
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let tls = tls_connector();
    let addr = ADDR.parse()?;

    // Clearnet baseline
    info!("Fetching https://{HOST}{PATH} via clearnet...");
    let clearnet_start = tokio::time::Instant::now();
    let clearnet_tcp = tokio::net::TcpStream::connect(addr).await?;
    let (clearnet_status, clearnet_body) = https_get(clearnet_tcp, &tls, HOST, PATH).await?;
    let clearnet_duration = clearnet_start.elapsed();
    info!("Clearnet: {} in {:?}", clearnet_status, clearnet_duration);

    // Mixnet via smolmix
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

    info!("Fetching https://{HOST}{PATH} via mixnet...");
    let mixnet_start = tokio::time::Instant::now();
    let mixnet_tcp = tunnel.tcp_connect(addr).await?;
    let (_mixnet_status, mixnet_body) = https_get(mixnet_tcp, &tls, HOST, PATH).await?;
    let mixnet_duration = mixnet_start.elapsed();

    // Compare
    info!("Results");
    let clearnet_ip = clearnet_body.lines().find(|l| l.starts_with("ip="));
    let mixnet_ip = mixnet_body.lines().find(|l| l.starts_with("ip="));
    info!("Clearnet IP: {}", clearnet_ip.unwrap_or("?"));
    info!("Mixnet IP:   {}", mixnet_ip.unwrap_or("?"));

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
