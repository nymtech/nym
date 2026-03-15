// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Demonstrates the stream/message mode mutual exclusion guard.
//!
//! A `MixnetClient` operates in one of two modes: message mode (default)
//! or stream mode (activated by `open_stream()` or `listener()`). Once
//! stream mode is active, message-based methods like `send_plain_message`
//! return `Error::StreamModeActive`. This is a one-way transition — the
//! two modes share a single inbound channel from the gateway, so they
//! cannot coexist.
//!
//! This example shows:
//! - Using the message API before stream mode
//! - Activating stream mode via `listener()`
//! - Observing `StreamModeActive` errors on message sends
//! - `split_sender()` shares the mode flag (via `Arc<AtomicBool>`)
//!
//! Run with: cargo run --example stream_mode_guard

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sdk::Error;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let our_address = *client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Step 1: Message-based API works before stream mode is activated.
    println!("\nTesting message-based API (should work)");
    client
        .send_plain_message(our_address, "hello via message API")
        .await
        .unwrap();
    println!("Message sent successfully via message-based API");

    // Step 2: Activate stream mode by creating a listener.
    // This is a one-way transition — the two modes share a single inbound
    // channel from the gateway, so they cannot coexist.
    println!("\nActivating stream mode via listener()");
    let _listener = client.listener().unwrap();
    println!("Stream mode is now active");

    // Step 3: Message-based API now returns Error::StreamModeActive.
    println!("\nTesting message-based API again (should fail)");
    let result = client
        .send_plain_message(our_address, "this should fail")
        .await;

    match result {
        Err(Error::StreamModeActive) => {
            println!("Got expected error: StreamModeActive");
        }
        Err(e) => {
            println!("Got unexpected error: {e:?}");
        }
        Ok(()) => {
            println!("ERROR: send() should have failed but succeeded!");
        }
    }

    // Step 4: split_sender() shares the mode flag (Arc<AtomicBool>),
    // so it also returns StreamModeActive.
    println!("\nTesting split_sender (shares stream_mode flag)");
    let sender = client.split_sender();
    let result = sender
        .send_plain_message(our_address, "this should also fail")
        .await;

    match result {
        Err(Error::StreamModeActive) => {
            println!("Got expected error: StreamModeActive on split sender");
        }
        Err(e) => {
            println!("Got unexpected error: {e:?}");
        }
        Ok(()) => {
            println!("ERROR: split_sender.send() should have failed but succeeded!");
        }
    }

    client.disconnect().await;
}
