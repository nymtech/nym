use std::path::PathBuf;

use nym_issue_credential::utils;
use nym_sdk::bandwidth::BandwidthAcquireClient;
use nym_sdk::mixnet;
use nym_validator_client::nyxd::Coin;

#[tokio::main]
async fn main() {
    let amount = 1000000;
    let client_home_directory = PathBuf::from("");
    let mnemonic = String::from("very secret mnemonic");
    let recovery_dir = PathBuf::from("");

    nym_bin_common::logging::setup_logging();
    // right now, only sandbox has coconut setup
    // this should be run from the `sdk/rust/nym-sdk` directory
    dotenvy::from_path("../../../envs/sandbox.env").unwrap();

    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();

    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(sandbox_network.clone())
        .enable_credentials_mode()
        .build()
        .await
        .unwrap();

    let coin = Coin::new(
        amount as u128,
        &sandbox_network.chain_details.mix_denom.base,
    );

    let persistent_storage = utils::setup_persistent_storage(client_home_directory.clone()).await;
    let client =
        BandwidthAcquireClient::new(sandbox_network, mnemonic, &persistent_storage).unwrap();
    utils::issue_credential(client.client, coin, client_home_directory, recovery_dir).await;

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
