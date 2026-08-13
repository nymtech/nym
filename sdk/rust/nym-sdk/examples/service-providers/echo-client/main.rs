//! Client for the echo service provider example.
//!
//! Opens a stream to a running `echo-service` (see the sibling example)
//! identified by its nym address, sends a single request, waits for the
//! JSON reply, prints it, and exits. SURBs for the anonymous reply path are
//! attached when the stream is opened, so the service can answer without
//! ever learning this client's address.
//!
//! ## What this demonstrates
//!
//! - Addressing a service provider by its nym address
//!   (`Recipient::try_from_base58_string`)
//! - `client.open_stream(recipient, surbs)` opens an outbound stream;
//!   [`MixnetStream`] implements `AsyncRead + AsyncWrite`, so standard
//!   tokio I/O (`write_all`, `read`) works unchanged
//! - The same explicit privacy configuration as the service: cover traffic
//!   and Poisson timing obfuscation on, free mode (no zk-nym credentials)
//!
//! ```sh
//! # first start the service and note the address it prints:
//! # cargo run --example echo-service
//! cargo run --example echo-client -- <service-nym-address>
//! ```

use nym_sdk::mixnet::{MixnetClientBuilder, Recipient};
use nym_sdk::DebugConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// How long to wait for the reply to make it back through the mixnet.
const REPLY_TIMEOUT: Duration = Duration::from_secs(120);

/// The reply the echo service sends. Mirrors the definition in the
/// echo-service example — keep the two in sync.
#[derive(Serialize, Deserialize)]
struct EchoResponse {
    message: String,
    timestamp_utc: String,
    request_id: String,
}

/// The same explicit privacy configuration as the echo service: keep the
/// Poisson packet stream (timing obfuscation) and the loop cover traffic
/// stream (unobservability) enabled. These are the defaults — see the
/// echo-service example for the full discussion.
fn privacy_config() -> DebugConfig {
    let mut debug_config = DebugConfig::default();
    // Real messages leave at randomized Poisson intervals (on average every
    // `traffic.message_sending_average_delay`), cover packets fill the gaps.
    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = false;
    // Dummy self-addressed packets flow continuously (on average every
    // `cover_traffic.loop_cover_traffic_average_delay`), hiding whether we
    // are communicating at all.
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = false;
    // Each packet is also delayed at every mix hop by a randomized amount
    // averaging `traffic.average_packet_delay`, defeating timing correlation.
    debug_config
}

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    let Some(raw_address) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example echo-client -- <service-nym-address>");
        eprintln!("start `cargo run --example echo-service` first and use the address it prints");
        std::process::exit(1);
    };
    let service_address = match Recipient::try_from_base58_string(&raw_address) {
        Ok(address) => address,
        Err(err) => {
            eprintln!("'{raw_address}' is not a valid nym address: {err}");
            std::process::exit(1);
        }
    };

    // Ephemeral keys and free mode (credentials mode is opt-in and not
    // enabled), matching the echo-service example.
    let client = MixnetClientBuilder::new_ephemeral()
        .debug_config(privacy_config())
        .build()
        .unwrap();
    let mut client = client.connect_to_mixnet().await.unwrap();
    println!("connected as {}", client.nym_address());

    // Open a stream to the service. `None` attaches the default number of
    // reply SURBs, which is what lets the service answer anonymously.
    println!("opening stream to {service_address}");
    let mut stream = client.open_stream(service_address, None).await.unwrap();
    println!("stream opened: {}", stream.id());

    stream.write_all(b"echo request").await.unwrap();
    stream.flush().await.unwrap();

    println!("waiting for reply");
    let mut buf = vec![0u8; 1024];
    let n = match tokio::time::timeout(REPLY_TIMEOUT, stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => n,
        Ok(Ok(_)) => {
            eprintln!("service closed the stream without replying");
            std::process::exit(1);
        }
        Ok(Err(err)) => {
            eprintln!("failed to read reply: {err}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("timed out waiting for a reply after {REPLY_TIMEOUT:?}");
            std::process::exit(1);
        }
    };

    let raw = String::from_utf8_lossy(&buf[..n]);
    let response: EchoResponse = match serde_json::from_str(&raw) {
        Ok(response) => response,
        Err(err) => {
            eprintln!("could not parse reply as JSON ({err}): {raw}");
            std::process::exit(1);
        }
    };
    // This exact prefix is parsed by tests/echo_example_integration.rs.
    println!("received reply: {raw}");
    println!("  message:       {}", response.message);
    println!("  timestamp_utc: {}", response.timestamp_utc);
    println!("  request_id:    {}", response.request_id);

    // Streams deregister on drop; no close handshake is needed.
    drop(stream);
    client.disconnect().await;
}
