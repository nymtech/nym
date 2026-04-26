//! Minimal message example: send a message to yourself and print it.
//!
//! Uses an ephemeral client — keys are generated in memory and discarded
//! on disconnect.
//!
//! ## What this demonstrates
//!
//! - `MixnetClient::connect_new()` creates an ephemeral client (ed25519 +
//!   x25519 keypair, gateway selection, topology fetch — all automatic)
//! - `send_plain_message()` wraps the payload in Sphinx packets and queues
//!   it for sending — encryption and mixing happen in background tasks
//! - `on_messages()` drains the inbound queue fed by the gateway
//!
//! For persistent identity (same address across restarts), see
//! `builder_with_storage.rs`. For anonymous replies, see `surb_reply.rs`.
//!
//! ```sh
//! cargo run --example simple
//! ```

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Connect an ephemeral client (keys generated in memory).
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message to ourselves through the mixnet.
    // The message is Sphinx-encrypted and routed through 5 nodes
    // (gateway → 3 mix nodes → gateway).
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    // Wait for incoming messages and print them.
    // on_messages blocks forever — press ctrl-c to exit.
    println!("Waiting for message (ctrl-c to exit)");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
