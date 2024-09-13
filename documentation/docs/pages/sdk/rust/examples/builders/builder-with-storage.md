# Mixnet Client Builder with Storage

The previous example involves ephemeral keys - if we want to create and then maintain a client identity over time, our code becomes a little more complex as we need to create, store, and conditionally load these keys.

> You can find this code [here](https://github.com/nymtech/nym/blob/master/sdk/rust/nym-sdk/examples/builder_with_storage.rs).

```rust
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    // Specify some config options
    let config_dir = PathBuf::from("/tmp/mixnet-client");
    let storage_paths = mixnet::StoragePaths::new_from_dir(&config_dir).unwrap();

    // Create the client with a storage backend, and enable it by giving it some paths. If keys
    // exists at these paths, they will be loaded, otherwise they will be generated.
    let client = mixnet::MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .unwrap();

    // Now we connect to the mixnet, using keys now stored in the paths provided.
    let mut client = client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message throught the mixnet to ourselves
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

As seen in the example above, the `mixnet::MixnetClientBuilder::new()` function handles checking for keys in a storage location, loading them if present, or creating them and storing them if not, making client key management very simple.

Assuming our client config is stored in `/tmp/mixnet-client`, the following files are generated:
```
$ tree /tmp/mixnet-client

mixnet-client
├── ack_key.pem
├── db.sqlite
├── db.sqlite-shm
├── db.sqlite-wal
├── gateway_details.json
├── gateway_shared.pem
├── persistent_reply_store.sqlite
├── private_encryption.pem
├── private_identity.pem
├── public_encryption.pem
└── public_identity.pem

1 directory, 11 files
```
