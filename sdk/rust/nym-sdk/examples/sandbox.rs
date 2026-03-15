//! Connecting to the Sandbox testnet instead of mainnet.
//!
//! Loads a sandbox `.env` file to override the default (mainnet)
//! network details, then sends a message to self.
//!
//! Run with: cargo run --example sandbox

use futures::StreamExt;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    // Step 1: Load the sandbox environment.
    // Set NYM_ENV_PATH or fall back to the in-repo env file.
    let env_path =
        std::env::var("NYM_ENV_PATH").unwrap_or_else(|_| "../../../envs/sandbox.env".to_string());
    setup_env(Some(&env_path));

    // Step 2: Build and connect using the sandbox network details.
    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();
    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(sandbox_network)
        .build()?;
    let mut client = mixnet_client.connect_to_mixnet().await?;
    let our_address = client.nym_address();

    // Step 3: Send a message to ourselves through the sandbox mixnet.
    client
        .send_plain_message(*our_address, "hello there")
        .await?;

    // Step 4: Wait for the message via the futures::Stream impl.
    println!("Waiting for message");
    if let Some(received) = client.next().await {
        println!("Received: {}", String::from_utf8_lossy(&received.message));
    } else {
        eprintln!("Failed to receive message.");
    }

    // Step 5: Disconnect for clean shutdown.
    client.disconnect().await;
    Ok(())
}
