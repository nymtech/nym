//! Using `MixnetClientBuilder` with persistent on-disk key storage.
//!
//! Keys are generated on the first run, then loaded from disk on
//! subsequent runs so the client keeps the same Nym address.
//!
//! Run with: cargo run --example builder_with_storage

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Step 1: Point storage at a directory.
    // If keys exist there they are loaded; otherwise new ones are generated.
    let config_dir = PathBuf::from("/tmp/mixnet-client");
    let storage_paths = mixnet::StoragePaths::new_from_dir(&config_dir).unwrap();

    // Step 2: Build the client with on-disk persistent storage.
    let client = mixnet::MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .unwrap();

    // Step 3: Connect to the mixnet.
    let mut client = client.connect_to_mixnet().await.unwrap();
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Step 4: Send a message to ourselves and wait for it.
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

    // Step 5: Always disconnect for clean shutdown.
    client.disconnect().await;
}
