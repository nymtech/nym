use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();
    // right now, only sandbox has coconut setup
    // this should be run from the `sdk/rust/nym-sdk` directory
    dotenvy::from_path("../../../envs/sandbox.env").unwrap();

    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();

    let mixnet_client = mixnet::MixnetClientBuilder::new()
        .network_details(sandbox_network)
        .enable_credentials_mode()
        .build::<mixnet::EmptyReplyStorage>()
        .await
        .unwrap();

    let bandwidth_client = mixnet_client
        .create_bandwidth_client(String::from("very secret mnemonic"))
        .unwrap();

    // Get a bandwidth credential worth 1000000 unym for the mixnet_client
    bandwidth_client.acquire(1000000).await.unwrap();

    // Connect using paid bandwidth credential
    let mut client = mixnet_client.connect_to_mixnet().await.unwrap();

    let our_address = client.nym_address();

    // Send a message throughout the mixnet to ourselves
    client.send_str(*our_address, "hello there").await;

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
        }
    }

    client.disconnect().await;
}
