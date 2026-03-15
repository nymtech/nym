//! Using `MixnetClientBuilder` with ephemeral (in-memory) keys.
//!
//! The builder lets you configure the client before connecting.
//! Ephemeral keys are generated in memory and discarded on disconnect.
//!
//! Run with: cargo run --example builder

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Step 1: Create a builder with ephemeral keys.
    // The builder lets you configure the client before connecting.
    let client = mixnet::MixnetClientBuilder::new_ephemeral()
        .build()
        .unwrap();

    // Step 2: Connect to the mixnet.
    let mut client = client.connect_to_mixnet().await.unwrap();
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Step 3: Send a message and wait for it.
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
        }
    }

    // Step 4: Always disconnect for clean shutdown.
    client.disconnect().await;
}
