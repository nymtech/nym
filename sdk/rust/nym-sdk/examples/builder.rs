use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    logging::setup_logging();

    // Create client builder, including ephemeral keys. The builder can be usable in the context
    // where you don't want to connect just yet.
    // Since not storage paths are given, the surb storage will be inactive.
    let client = mixnet::MixnetClientBuilder::new()
        .build::<mixnet::EmptyReplyStorage>()
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
