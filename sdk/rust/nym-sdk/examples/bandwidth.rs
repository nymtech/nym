use futures::StreamExt;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_logging();
    // right now, only sandbox has coconut setup
    // this should be run from the `sdk/rust/nym-sdk` directory
    setup_env(Some("../../../envs/sandbox.env"));

    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();
    let mnemonic = String::from("my super secret mnemonic");

    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(sandbox_network)
        .enable_credentials_mode()
        .build()?;

    let bandwidth_client = mixnet_client.create_bandwidth_client(mnemonic).await?;

    // Get a bandwidth credential for the mixnet_client
    bandwidth_client.acquire().await?;

    // Connect using paid bandwidth credential
    let mut client = mixnet_client.connect_to_mixnet().await?;

    let our_address = client.nym_address();

    // Send a message throughout the mixnet to ourselves
    client
        .send_plain_message(*our_address, "hello there")
        .await?;

    println!("Waiting for message");
    let received = client.next().await.unwrap();
    println!("Received: {}", String::from_utf8_lossy(&received.message));

    client.disconnect().await;
    Ok(())
}
