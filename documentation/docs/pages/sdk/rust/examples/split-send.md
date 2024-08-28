# Send and Receive in Different Tasks
If you need to split the different actions of your client across different tasks, you can do so like this. You can think of this analogously to spliting a Tcp Stream into read/write. This functionality is also useful for embedding a sending and receiving client into different tasks.

> You can find this code [here](https://github.com/nymtech/nym/blob/master/sdk/rust/nym-sdk/examples/parallel_sending_and_receiving.rs)

```rust
// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    // Passing no config makes the client fire up an ephemeral session and figure stuff out on its own
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();

    // Be able to get our client address
    let our_address = *client.nym_address();
    println!("Our client nym address is: {our_address}");

    let sender = client.split_sender();

    // receiving task
    let receiving_task_handle = tokio::spawn(async move {
        if let Some(received) = client.next().await {
            println!("Received: {}", String::from_utf8_lossy(&received.message));
        }

        client.disconnect().await;
    });

    // sending task
    let sending_task_handle = tokio::spawn(async move {
        sender
            .send_plain_message(our_address, "hello from a different task!")
            .await
            .unwrap();
    });

    // wait for both tasks to be done
    println!("waiting for shutdown");
    sending_task_handle.await.unwrap();
    receiving_task_handle.await.unwrap();
}
```
