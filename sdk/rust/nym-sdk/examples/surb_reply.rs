//! Anonymous replies using SURBs (Single Use Reply Blocks).
//!
//! Sends a message to self, extracts the `AnonymousSenderTag` from the
//! incoming message, and replies using `send_reply()` without knowing
//! the sender's Nym address. The SDK bundles SURBs with every outgoing
//! message by default, so the recipient can always reply anonymously.
//!
//! ## What this demonstrates
//!
//! - Every incoming message carries a `sender_tag`, an opaque
//!   [`AnonymousSenderTag`] that enables replies without revealing the
//!   sender's address
//! - `send_reply()` consumes a SURB to route the reply back through the
//!   mixnet. Each SURB is single-use; the SDK replenishes them automatically
//! - This is the foundation for anonymous communication: the server never
//!   learns who is talking to it
//!
//! ```sh
//! cargo run --example surb_reply
//! ```

use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClientBuilder, MixnetMessageSender, ReconstructedMessage,
    StoragePaths,
};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Build a client with persistent key storage.
    // Keys are generated on first run, then loaded from disk on subsequent runs.
    let config_dir: PathBuf = TempDir::new().unwrap().path().to_path_buf();
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .unwrap();
    let mut client = client.connect_to_mixnet().await.unwrap();
    let our_address = client.nym_address();
    println!("\nOur client nym address is: {our_address}");

    // Send a message to ourselves.
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    // Receive the message.
    println!("Waiting for message\n");
    let mut message: Vec<ReconstructedMessage> = Vec::new();
    // Filter empty messages: these are SURB replenishment requests.
    while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            continue;
        }
        message = new_message;
        break;
    }

    let parsed = String::from_utf8(message[0].message.clone()).unwrap();

    // Extract the AnonymousSenderTag from the incoming message.
    // This opaque token lets you reply without knowing the sender's address.
    // The SDK includes SURBs with every message by default.
    let return_recipient: AnonymousSenderTag = message[0].sender_tag.unwrap();
    println!("Received: {parsed}\nSender tag: {return_recipient}");

    // Reply anonymously using send_reply() instead of send_plain_message().
    println!("Replying using SURBs...");
    client
        .send_reply(return_recipient, "hi an0n!")
        .await
        .unwrap();

    // Receive the reply.
    println!("Waiting for reply (ctrl-c to exit)\n");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
