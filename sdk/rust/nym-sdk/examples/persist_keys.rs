use std::path::PathBuf;

use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    logging::setup_logging();

    // Specify some config options
    let config_dir = PathBuf::from("/tmp/mixnet-client");

    // Setting `KeyMode::Keep` will use existing keys, and existing config, if there is one.
    // Regardles of `user_chosen_gateway`.
    let keys = mixnet::StoragePaths::new_from_dir(mixnet::KeyMode::Keep, &config_dir).unwrap();

    // Provide key paths for the client to read/write keys to.
    let client = mixnet::MixnetClientBuilder::new(None, Some(keys))
        .await
        .unwrap();

    // Connect to the mixnet, now we're listening for incoming
    let mut client = client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message throught the mixnet to ourselves
    client
        .send_str(&our_address.to_string(), "hello there")
        .await;

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
        }
    }

    client.disconnect().await;
}
