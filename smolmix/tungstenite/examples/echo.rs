//! WebSocket echo: clearnet vs Nym mixnet comparison.
//!
//! Sends a message to a public echo server over clearnet (tokio-tungstenite
//! directly) and through the mixnet (smolmix-tungstenite), then compares
//! responses and timing.
//!
//! Run with:
//!   cargo run -p smolmix-tungstenite --example echo
//!   cargo run -p smolmix-tungstenite --example echo -- --ipr <IPR_ADDRESS>

use futures::{SinkExt, StreamExt};
use smolmix::{Recipient, Tunnel};
use smolmix_tungstenite::Message;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const WS_URL: &str = "wss://ws.postman-echo.com/raw";
const ECHO_MSG: &str = "Hello from the Nym mixnet!";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    smolmix::init_logging();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // --- Clearnet baseline via tokio-tungstenite ---
    info!("Connecting via clearnet...");
    let clearnet_start = tokio::time::Instant::now();

    let (mut clearnet_ws, _) = tokio_tungstenite::connect_async(WS_URL).await?;
    clearnet_ws.send(Message::Text(ECHO_MSG.into())).await?;
    let clearnet_reply = clearnet_ws.next().await.ok_or("no clearnet reply")??;
    let clearnet_duration = clearnet_start.elapsed();
    let clearnet_text = clearnet_reply.into_text()?;
    clearnet_ws.close(None).await?;

    info!("Clearnet: \"{clearnet_text}\" in {clearnet_duration:?}");

    // --- Mixnet via smolmix-tungstenite ---
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

    let mixnet_start = tokio::time::Instant::now();
    let (mut mixnet_ws, _) = smolmix_tungstenite::connect(&tunnel, WS_URL).await?;
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
