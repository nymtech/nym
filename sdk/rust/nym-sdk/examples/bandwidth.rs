//! Paid bandwidth credentials using the Ecash scheme.
//!
//! Acquires a bandwidth credential from the sandbox network, connects
//! with it, and sends a message to self. Requires the sandbox `.env`.
//!
//! Run with: cargo run --example bandwidth

use futures::StreamExt;
use nym_credentials_interface::TicketType;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    // Load the sandbox environment.
    // Run from the `sdk/rust/nym-sdk` directory so the relative path resolves.
    setup_env(Some("../../../envs/sandbox.env"));

    // Build a credentials-enabled client (not yet connected).
    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();
    let mnemonic = String::from("my super secret mnemonic");

    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(sandbox_network)
        .enable_credentials_mode()
        .build()?;

    // Acquire a bandwidth credential (ticketbook) before connecting.
    let bandwidth_client = mixnet_client
        .create_bandwidth_client(mnemonic, TicketType::V1MixnetEntry)
        .await?;
    bandwidth_client.acquire().await?;

    // Connect to the mixnet using the acquired credential.
    let mut client = mixnet_client.connect_to_mixnet().await?;
    let our_address = client.nym_address();

    // Send a message to ourselves and wait for it.
    client
        .send_plain_message(*our_address, "hello there")
        .await?;

    println!("Waiting for message");
    let received = client.next().await.unwrap();
    println!("Received: {}", String::from_utf8_lossy(&received.message));

    // Disconnect for clean shutdown.
    client.disconnect().await;
    Ok(())
}
