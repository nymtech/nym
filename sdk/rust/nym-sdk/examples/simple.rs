//! Minimal message example: send a message to yourself and print it.
//!
//! Uses an ephemeral client — no keys are stored to disk.
//!
//! Run with: cargo run --example simple

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Step 1: Connect an ephemeral client (keys generated in memory).
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Step 2: Send a message to ourselves through the mixnet.
    // The message is Sphinx-encrypted and mixed across 5 nodes.
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    // Step 3: Wait for incoming messages and print them.
    // on_messages blocks forever — press ctrl-c to exit.
    println!("Waiting for message (ctrl-c to exit)");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
