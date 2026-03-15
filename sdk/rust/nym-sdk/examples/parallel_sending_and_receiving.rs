// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sending and receiving from separate tasks using `split_sender()`.
//!
//! `split_sender()` returns a clone-able `MixnetClientSender` that can
//! send messages from any task, while the original client handles
//! receiving via `futures::Stream`.
//!
//! Run with: cargo run --example parallel_sending_and_receiving

use futures::StreamExt;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Step 1: Connect an ephemeral client.
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let our_address = *client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Step 2: Split the client into a sender handle.
    // The sender is clone-able and can be moved into any task.
    let sender = client.split_sender();

    // Step 3: Spawn a receiving task.
    // The original client implements futures::Stream, so you can use .next().
    let receiving_task_handle = tokio::spawn(async move {
        if let Some(received) = client.next().await {
            println!("Received: {}", String::from_utf8_lossy(&received.message));
        }
        client.disconnect().await;
    });

    // Step 4: Spawn a sending task using the split sender.
    let sending_task_handle = tokio::spawn(async move {
        sender
            .send_plain_message(our_address, "hello from a different task!")
            .await
            .unwrap();
    });

    // Step 5: Wait for both tasks to complete.
    println!("waiting for shutdown");
    sending_task_handle.await.unwrap();
    receiving_task_handle.await.unwrap();
}
