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
