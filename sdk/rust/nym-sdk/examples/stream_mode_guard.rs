// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sdk::Error;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let our_address = *client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Message-based API works before stream mode is activated
    println!("\nTesting message-based API (should work)");
    client
        .send_plain_message(our_address, "hello via message API")
        .await
        .unwrap();
    println!("Message sent successfully via message-based API");

    // Now activate stream mode by creating a listener
    println!("\nActivating stream mode via listener()");
    let _listener = client.listener().unwrap();
    println!("Stream mode is now active");

    // Message-based API should now fail
    println!("\nTesting message-based API again (should fail)");
    let result = client
        .send_plain_message(our_address, "this should fail")
        .await;

    match result {
        Err(Error::StreamModeActive) => {
            println!("Got expected error: StreamModeActive");
            println!("Message-based API correctly blocked after stream mode activation");
        }
        Err(e) => {
            println!("Got unexpected error: {e:?}");
        }
        Ok(()) => {
            println!("ERROR: send() should have failed but succeeded!");
        }
    }

    // split_sender shares the stream_mode flag
    println!("\nTesting split_sender (shares stream_mode)");
    let sender = client.split_sender();
    let result = sender
        .send_plain_message(our_address, "this should also fail")
        .await;

    match result {
        Err(Error::StreamModeActive) => {
            println!("Got expected error: StreamModeActive on split sender");
            println!("Split sender correctly shares stream_mode with parent client");
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
