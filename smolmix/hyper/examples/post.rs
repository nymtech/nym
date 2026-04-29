//! HTTP POST: clearnet vs Nym mixnet comparison.
//!
//! Sends a POST request with a JSON body to httpbin.org via clearnet (reqwest)
//! and through the mixnet (smolmix-hyper with SmolmixConnector), then compares
//! responses and timing.
//!
//! Demonstrates using `SmolmixConnector` directly for requests that carry a body
//! (the `Client` newtype uses `Empty<Bytes>` — for POST you build a custom client).
//!
//! Run with:
//!   cargo run -p smolmix-hyper --example post
//!   cargo run -p smolmix-hyper --example post -- --ipr <IPR_ADDRESS>

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use smolmix::{Recipient, Tunnel};
use smolmix_hyper::SmolmixConnector;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const URL: &str = "https://httpbin.org/post";
const JSON_BODY: &str = r#"{"message": "hello from the Nym mixnet!"}"#;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Clearnet baseline via reqwest
    info!("POST via clearnet...");
    let clearnet_start = tokio::time::Instant::now();
    let clearnet_resp = reqwest::Client::new()
        .post(URL)
        .header("Content-Type", "application/json")
        .body(JSON_BODY)
        .send()
        .await?;
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

    // For POST requests, use SmolmixConnector directly with a Full<Bytes> body
    let connector = SmolmixConnector::new(&tunnel);
    let client: Client<SmolmixConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(connector);

    info!("POST via mixnet...");
    let mixnet_start = tokio::time::Instant::now();
    let req = Request::post(URL)
        .header("Host", "httpbin.org")
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(JSON_BODY)))?;
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

    // Check that the echo'd body contains our message
    let clearnet_has_msg = clearnet_body.contains("hello from the Nym mixnet!");
    let mixnet_has_msg = mixnet_body.contains("hello from the Nym mixnet!");
    info!("Body echo clearnet: {clearnet_has_msg}");
    info!("Body echo mixnet:   {mixnet_has_msg}");

    let slowdown = mixnet_duration.as_millis() as f64 / clearnet_duration.as_millis().max(1) as f64;
    info!("Slowdown: {slowdown:.1}x");

    tunnel.shutdown().await;
    Ok(())
}
