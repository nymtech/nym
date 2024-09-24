use futures::StreamExt;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

// An example of creating a client relying on a testnet, in this case Sandbox.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_logging();
    // relative root is `sdk/rust/nym-sdk/` for fallback file path
    let env_path =
        std::env::var("NYM_ENV_PATH").unwrap_or_else(|_| "../../../envs/sandbox.env".to_string());
    setup_env(Some(&env_path));
    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();

    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(sandbox_network)
        .build()?;

    let mut client = mixnet_client.connect_to_mixnet().await?;

    let our_address = client.nym_address();

    // Send a message throughout the mixnet to ourselves
    client
        .send_plain_message(*our_address, "hello there")
        .await?;

    println!("Waiting for message");
    if let Some(received) = client.next().await {
        println!("Received: {}", String::from_utf8_lossy(&received.message));
    } else {
        eprintln!("Failed to receive message.");
    }

    client.disconnect().await;
    Ok(())
}
