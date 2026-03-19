//! HTTPS GET through the Nym mixnet using the Tunnel API.
//!
//! Fetches Cloudflare's `/cdn-cgi/trace` endpoint over clearnet (reqwest) and
//! through the mixnet (hyper over tokio-rustls over smolmix), then compares
//! responses and timing.
//!
//! Run with:
//!   cargo run --example tunnel_https

use std::sync::Arc;

use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::Request;
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use smolmix::{NetworkEnvironment, Tunnel};
use tokio_rustls::TlsConnector;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    smolmix::init_logging();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let host = "cloudflare.com";
    let path = "/cdn-cgi/trace";

    // --- Clearnet baseline via reqwest ---
    info!("Fetching via clearnet...");
    let clearnet_start = tokio::time::Instant::now();
    let clearnet_resp = reqwest::get(format!("https://{host}{path}")).await?;
    let clearnet_status = clearnet_resp.status();
    let clearnet_body = clearnet_resp.text().await?;
    let clearnet_duration = clearnet_start.elapsed();
    info!("Clearnet: {} in {:?}", clearnet_status, clearnet_duration);

    // --- Mixnet via tunnel + tokio-rustls + hyper ---
    let tunnel = Tunnel::new(NetworkEnvironment::Mainnet).await?;

    let mixnet_start = tokio::time::Instant::now();
    let tcp = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;

    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(tls_config));
    let domain = ServerName::try_from(host)?.to_owned();
    let tls = connector.connect(domain, tcp).await?;

    // Hand the TLS stream to hyper for proper HTTP/1.1 handling.
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(tls)).await?;
    tokio::spawn(conn);

    let req = Request::get(path)
        .header("Host", host)
        .body(http_body_util::Empty::<Bytes>::new())?;
    let resp = sender.send_request(req).await?;

    let mixnet_status = resp.status();
    let body_bytes = resp.into_body().collect().await?.to_bytes();
    let mixnet_body = String::from_utf8_lossy(&body_bytes);
    let mixnet_duration = mixnet_start.elapsed();

    // --- Compare ---
    info!("=== Results ===");
    info!("Clearnet: {} in {:?}", clearnet_status, clearnet_duration);
    info!("Mixnet:   {} in {:?}", mixnet_status, mixnet_duration);
    info!("Status match: {}", clearnet_status == mixnet_status);

    // Both should contain the same diagnostic fields.
    let fields = ["fl=", "visit_scheme=https", "uag="];
    for field in fields {
        let clearnet_has = clearnet_body.contains(field);
        let mixnet_has = mixnet_body.contains(field);
        info!("  {field:<25} clearnet={clearnet_has}  mixnet={mixnet_has}");
    }

    // The IP should differ (mixnet uses an exit node).
    let clearnet_ip = clearnet_body.lines().find(|l| l.starts_with("ip="));
    let mixnet_ip = mixnet_body.lines().find(|l| l.starts_with("ip="));
    info!("  Clearnet IP: {}", clearnet_ip.unwrap_or("?"));
    info!("  Mixnet IP:   {}", mixnet_ip.unwrap_or("?"));

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("  Slowdown:    {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
