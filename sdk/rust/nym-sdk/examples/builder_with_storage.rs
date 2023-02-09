use std::path::PathBuf;

use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    logging::setup_logging();

    // Specify some config options
    let config_dir = PathBuf::from("/tmp/mixnet-client");
    let storage_paths =
        mixnet::StoragePaths::new_from_dir(mixnet::KeyMode::Keep, &config_dir).unwrap();

    // Create the client with a storage backend, and enable it by giving it some paths
    let client = mixnet::MixnetClientBuilder::new()
        .enable_storage(storage_paths)
        .build::<mixnet::ReplyStorage>()
        .await
        .unwrap();

    // Now we connect to the mixnet, using ephemeral keys already created
    let mut client = client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send a message throught the mixnet to ourselves
    client.send_str(*our_address, "hello there").await;

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
        }
    }

    client.disconnect().await;
}
