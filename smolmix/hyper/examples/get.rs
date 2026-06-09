//! HTTPS GET: clearnet vs Nym mixnet comparison.
//!
//! Fetches Cloudflare's `/cdn-cgi/trace` endpoint over clearnet (reqwest) and
//! through the mixnet (smolmix-hyper), then compares responses and timing.
//!
//! Run with:
//!   cargo run -p smolmix-hyper --example get
//!   cargo run -p smolmix-hyper --example get -- --ipr <IPR_ADDRESS>

use smolmix::{Recipient, Tunnel};
use smolmix_hyper::{BodyExt, Client, EmptyBody, Request};
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let host = "cloudflare.com";
    let path = "/cdn-cgi/trace";

    // Clearnet baseline via reqwest
    info!("Fetching via clearnet...");
    let clearnet_start = tokio::time::Instant::now();
    let clearnet_resp = reqwest::get(format!("https://{host}{path}")).await?;
    let clearnet_status = clearnet_resp.status();
    let clearnet_body = clearnet_resp.text().await?;
    let clearnet_duration = clearnet_start.elapsed();
    info!("Clearnet: {} in {:?}", clearnet_status, clearnet_duration);

    // Mixnet via smolmix-hyper
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

    let client = Client::new(&tunnel);
    let mixnet_start = tokio::time::Instant::now();

    let req = Request::get(format!("https://{host}{path}"))
        .header("Host", host)
        .body(EmptyBody::<bytes::Bytes>::new())?;
    let resp = client.request(req).await?;

    let mixnet_status = resp.status();
    let body_bytes = resp.into_body().collect().await?.to_bytes();
    let mixnet_body = String::from_utf8_lossy(&body_bytes);
    let mixnet_duration = mixnet_start.elapsed();

    // Compare
    info!("Results");
    info!("Clearnet: {} in {:?}", clearnet_status, clearnet_duration);
    info!("Mixnet:   {} in {:?}", mixnet_status, mixnet_duration);
    info!("Status match: {}", clearnet_status == mixnet_status);

    let fields = ["fl=", "visit_scheme=https", "uag="];
    for field in fields {
        let clearnet_has = clearnet_body.contains(field);
        let mixnet_has = mixnet_body.contains(field);
        info!("  {field:<25} clearnet={clearnet_has}  mixnet={mixnet_has}");
    }

    let clearnet_ip = clearnet_body.lines().find(|l| l.starts_with("ip="));
    let mixnet_ip = mixnet_body.lines().find(|l| l.starts_with("ip="));
    info!("  Clearnet IP: {}", clearnet_ip.unwrap_or("?"));
    info!("  Mixnet IP:   {}", mixnet_ip.unwrap_or("?"));

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("  Slowdown:    {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
