//! The complete code of the mixnet tutorial, persistent identity in the docs
//! (documentation/docs/.nym-codex/snippets/developers/rust/mixnet/tutorial/persistent-identity.mdx).
//! Keep the two the same: a docs test compares them.
//!
//! ```sh
//! cargo run --example tutorial_mixnet_persistent
//! ```

use nym_sdk::mixnet::{self, MixnetMessageSender, StoragePaths};

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    let paths = StoragePaths::new_from_dir("./my-client-data").unwrap();

    let mut client = mixnet::MixnetClientBuilder::new_with_default_storage(paths)
        .await
        .unwrap()
        .build()
        .unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

    let our_address = client.nym_address();
    println!("Address: {our_address}");

    client
        .send_plain_message(*our_address, "hello from persistent client!")
        .await
        .unwrap();
    println!("Sent, waiting for arrival...");

    let message = loop {
        if let Some(msgs) = client.wait_for_messages().await {
            if let Some(msg) = msgs.into_iter().find(|m| !m.message.is_empty()) {
                break msg;
            }
        }
    };
    println!("Received: {}", String::from_utf8_lossy(&message.message));

    let sender_tag = message.sender_tag.expect("should have sender tag");
    client
        .send_reply(sender_tag, "anonymous reply!")
        .await
        .unwrap();

    let reply = loop {
        if let Some(msgs) = client.wait_for_messages().await {
            if let Some(msg) = msgs.into_iter().find(|m| !m.message.is_empty()) {
                break msg;
            }
        }
    };
    println!("Reply: {}", String::from_utf8_lossy(&reply.message));

    client.disconnect().await;
}
