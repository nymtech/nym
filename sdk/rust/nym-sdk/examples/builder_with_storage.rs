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
