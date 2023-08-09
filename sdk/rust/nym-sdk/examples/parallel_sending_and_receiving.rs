// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    // Passing no config makes the client fire up an ephemeral session and figure stuff out on its own
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();

    // Be able to get our client address
    let our_address = *client.nym_address();
    println!("Our client nym address is: {our_address}");

    let receive_notify = Arc::new(Notify::new());
    let receive_notify2 = receive_notify.clone();
    let send_notify = Arc::new(Notify::new());
    let send_notify2 = send_notify.clone();

    let sender = client.split_sender();

    // receiving task
    tokio::spawn(async move {
        if let Some(received) = client.next().await {
            for r in received {
                println!("Received: {}", String::from_utf8_lossy(&r.message));
            }
        }

        client.disconnect().await;
        // notify that we're done and leave the task!
        receive_notify.notify_waiters();
    });

    // sending task
    tokio::spawn(async move {
        sender
            .send_plain_message(our_address, "hello from a different task!")
            .await
            .unwrap();

        send_notify.notify_waiters();
    });

    // wait for both tasks to be done
    println!("waiting for shutdown");
    send_notify2.notified().await;
    receive_notify2.notified().await;
}
