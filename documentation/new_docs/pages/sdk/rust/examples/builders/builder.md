# Mixnet Client Builder

> You can find this code [here](https://github.com/nymtech/nym/blob/master/sdk/rust/nym-sdk/examples/builder.rs).

```rust
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    // Create client builder, including ephemeral keys. The builder can be usable in the context
    // where you don't want to connect just yet.
    let client = mixnet::MixnetClientBuilder::new_ephemeral()
        .build()
        .unwrap();

    // Now we connect to the mixnet, using ephemeral keys already created
    let mut client = client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message through the mixnet to ourselves
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

    client.disconnect().await;
}
```
