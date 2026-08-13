//! Client for the echo service provider example.
//!
//! Sends a single request to a running `echo-service` (see the sibling
//! example) identified by its nym address, waits for the JSON reply, prints
//! it, and exits. The request travels through the mixnet with SURBs bundled
//! automatically, so the service can reply without ever learning this
//! client's address.
//!
//! ## What this demonstrates
//!
//! - Addressing a service provider by its nym address
//!   (`Recipient::try_from_base58_string`)
//! - `send_plain_message()` bundles reply SURBs by default — anonymous
//!   replies require no extra work on the client side
//! - The same explicit privacy configuration as the service: cover traffic
//!   and Poisson timing obfuscation on, free mode (no zk-nym credentials)
//!
//! ```sh
//! # first start the service and note the address it prints:
//! # cargo run --example echo-service
//! cargo run --example echo-client -- <service-nym-address>
//! ```

use nym_sdk::mixnet::{MixnetClientBuilder, MixnetMessageSender, Recipient};
use nym_sdk::DebugConfig;
use serde::{Deserialize, Serialize};

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
/// echo-service example for a field-by-field explanation.
fn privacy_config() -> DebugConfig {
    let mut debug_config = DebugConfig::default();
    debug_config.traffic.disable_main_poisson_packet_distribution = false;
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = false;
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

    // SURBs for the anonymous reply are bundled automatically.
    println!("sending echo request to {service_address}");
    client
        .send_plain_message(service_address, "echo request")
        .await
        .unwrap();

    println!("waiting for reply");
    'outer: while let Some(messages) = client.wait_for_messages().await {
        for message in messages {
            // Skip empty messages: SURB replenishment handled by the SDK.
            if message.message.is_empty() {
                continue;
            }
            let raw = String::from_utf8_lossy(&message.message);
            let response: EchoResponse = match serde_json::from_str(&raw) {
                Ok(response) => response,
                Err(err) => {
                    eprintln!("could not parse reply as JSON ({err}): {raw}");
                    std::process::exit(1);
                }
            };
            println!("received reply: {raw}");
            println!("  message:       {}", response.message);
            println!("  timestamp_utc: {}", response.timestamp_utc);
            println!("  request_id:    {}", response.request_id);
            break 'outer;
        }
    }

    client.disconnect().await;
}
