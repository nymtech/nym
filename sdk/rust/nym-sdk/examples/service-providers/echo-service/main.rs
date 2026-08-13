//! A minimal Nym service provider: an anonymous echo service.
//!
//! Runs as a long-lived service reachable over the mixnet. Every request
//! received is answered with a JSON payload containing a greeting, the
//! server time in UTC, and a random UUID as the response id. Replies are
//! sent using SURBs (Single Use Reply Blocks), so the service never learns
//! who is talking to it.
//!
//! ## What this demonstrates
//!
//! - A service provider is just a mixnet client with a request loop: it has
//!   a nym address, listens for messages, and replies. No gateway embedding
//!   or special registration is required
//! - Replies go through `send_reply()` with the `AnonymousSenderTag` taken
//!   from the incoming message. The client's real address is never revealed
//!   to the service
//! - The privacy knobs — cover traffic and Poisson timing obfuscation — are
//!   configured explicitly via [`DebugConfig`], with comments explaining
//!   what each one does. They are on by default; this example makes them
//!   visible
//! - The client runs in free mode (no zk-nym credentials), so this works on
//!   mainnet without holding NYM tokens
//!
//! ```sh
//! cargo run --example echo-service
//! # note the printed nym address, then in another terminal:
//! # cargo run --example echo-client -- <address>
//! ```

use nym_sdk::mixnet::{MixnetClientBuilder, MixnetMessageSender};
use nym_sdk::DebugConfig;
use serde::{Deserialize, Serialize};

/// The reply sent back for every request. The echo-client example parses
/// exactly this structure, so keep the two definitions in sync.
#[derive(Serialize, Deserialize)]
struct EchoResponse {
    message: String,
    timestamp_utc: String,
    request_id: String,
}

/// Build the client debug configuration with the mixnet privacy features
/// explicitly enabled. These are all the default values — a plain
/// `DebugConfig::default()` behaves identically — but a real service should
/// be deliberate about its traffic profile, so the knobs are spelled out.
fn privacy_config() -> DebugConfig {
    let mut debug_config = DebugConfig::default();

    // Keep the main Poisson packet stream enabled: real messages leave the
    // client at randomized intervals (on average every
    // `traffic.message_sending_average_delay`), with cover packets filling
    // the gaps. An observer sees a constant packet rate regardless of how
    // much the service is actually replying (timing obfuscation).
    debug_config.traffic.disable_main_poisson_packet_distribution = false;

    // Keep the loop cover traffic stream enabled: the client continuously
    // sends dummy packets addressed to itself (on average every
    // `cover_traffic.loop_cover_traffic_average_delay`), so an observer
    // cannot tell whether it is communicating at all (unobservability).
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = false;

    // Each packet is additionally delayed at every mix hop by a randomized
    // amount averaging `traffic.average_packet_delay`, which is what makes
    // input and output packets of a mix node uncorrelatable by timing.

    debug_config
}

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // An ephemeral client: keys live in memory and the nym address changes
    // every run. A real service provider should persist its keys so its
    // address stays stable — see the `builder_with_storage` example.
    //
    // Credentials mode is opt-in (`enable_credentials_mode()`); by not
    // enabling it the client runs in free mode, which the network currently
    // accepts for mixnet traffic — no NYM tokens needed.
    let client = MixnetClientBuilder::new_ephemeral()
        .debug_config(privacy_config())
        .build()
        .unwrap();
    let mut client = client.connect_to_mixnet().await.unwrap();

    let address = client.nym_address();
    println!("echo-service listening on: {address}");

    while let Some(messages) = client.wait_for_messages().await {
        for message in messages {
            // Empty messages are SURB replenishment requests handled by the
            // SDK; there is nothing to reply to.
            if message.message.is_empty() {
                continue;
            }
            // The sender tag is an opaque token identifying a bundle of
            // SURBs — pre-built reply routes the client sent along with its
            // request. It is all we ever learn about the requester.
            let Some(sender_tag) = message.sender_tag else {
                println!("received a message without SURBs attached - cannot reply");
                continue;
            };

            let request = String::from_utf8_lossy(&message.message);
            println!("received request: {request}");

            let response = EchoResponse {
                message: "hello".to_string(),
                timestamp_utc: chrono::Utc::now().to_rfc3339(),
                request_id: uuid::Uuid::new_v4().to_string(),
            };
            let json = serde_json::to_string(&response).unwrap();

            println!("replying via SURBs: {json}");
            client.send_reply(sender_tag, json).await.unwrap();
        }
    }
}
