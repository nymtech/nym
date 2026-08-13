//! A minimal Nym service provider: an anonymous echo service.
//!
//! Runs as a long-lived service reachable over the mixnet. Clients open
//! streams to it; every request received is answered with a JSON payload
//! containing a greeting, the server time in UTC, and a random UUID as the
//! response id. Replies travel back over the stream via SURBs (Single Use
//! Reply Blocks), so the service never learns who is talking to it.
//!
//! ## What this demonstrates
//!
//! - A service provider is just a mixnet client with an accept loop: it has
//!   a nym address, accepts incoming streams, and replies. No gateway
//!   embedding or special registration is required
//! - `client.listener()` activates stream mode; `listener.accept()` yields a
//!   [`MixnetStream`] per connecting client, which implements
//!   `AsyncRead + AsyncWrite` — the same traits as a TCP socket
//! - Writes on an accepted stream travel via SURBs bundled by the remote
//!   peer: the client's real address is never revealed to the service
//! - Each stream is handled in its own task, like a classic socket server
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

use nym_sdk::mixnet::{MixnetClientBuilder, MixnetStream};
use nym_sdk::DebugConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// How long a connected client gets to send its request.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

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
    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = false;

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

/// Serve one connected client: read its request, reply with the echo JSON.
/// The stream deregisters on drop; no close handshake is needed.
async fn handle_stream(mut stream: MixnetStream) {
    // We know nothing about the peer except its stream id — replies are
    // routed through the SURBs it attached when opening the stream.
    let stream_id = stream.id();

    let mut buf = vec![0u8; 1024];
    let n = match tokio::time::timeout(READ_TIMEOUT, stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => n,
        Ok(Ok(_)) => {
            println!("[{stream_id}] client closed the stream without sending a request");
            return;
        }
        Ok(Err(err)) => {
            println!("[{stream_id}] failed to read request: {err}");
            return;
        }
        Err(_) => {
            println!("[{stream_id}] timed out waiting for a request");
            return;
        }
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    println!("[{stream_id}] received request: {request}");

    let response = EchoResponse {
        message: "hello".to_string(),
        timestamp_utc: chrono::Utc::now().to_rfc3339(),
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();

    println!("[{stream_id}] replying via SURBs: {json}");
    if let Err(err) = stream.write_all(json.as_bytes()).await {
        println!("[{stream_id}] failed to write reply: {err}");
        return;
    }
    if let Err(err) = stream.flush().await {
        println!("[{stream_id}] failed to flush reply: {err}");
    }
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
    // This exact prefix is parsed by tests/echo_example_integration.rs.
    println!("echo-service listening on: {address}");

    // Activate stream mode. From here on, incoming streams arrive through
    // the listener, exactly like accepting connections on a TCP socket.
    let mut listener = client.listener().unwrap();

    while let Some(stream) = listener.accept().await {
        println!("accepted stream: {}", stream.id());
        // One task per client, like a classic socket server. The accept
        // loop keeps running while requests are served concurrently.
        tokio::spawn(handle_stream(stream));
    }
}
