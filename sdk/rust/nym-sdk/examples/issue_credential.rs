use std::path::PathBuf;

use nym_credential_utils::errors::Result;
use nym_credential_utils::utils;
use nym_sdk::mixnet::{self, MixnetClientStorage, MixnetMessageSender};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::{Client, Config};

#[tokio::main]
async fn main() -> Result<()> {
    let amount = 1000000;
    let mnemonic = String::from("");
    let client_home_directory = PathBuf::from("");
    let recovery_dir = PathBuf::from("");

    nym_bin_common::logging::setup_logging();
    // right now, only sandbox has coconut setup
    // this should be run from the `sdk/rust/nym-sdk` directory
    dotenvy::from_path("../../../envs/sandbox.env").unwrap();

    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();
    let config = Config::try_from_nym_network_details(&sandbox_network).unwrap();

    let coin = Coin::new(
        amount as u128,
        &sandbox_network.chain_details.mix_denom.base,
    );

    let storage_paths =
        mixnet::StoragePaths::new_from_dir(client_home_directory.as_path()).unwrap();
    let storage = storage_paths
        .initialise_default_persistent_storage()
        .await
        .unwrap();

    let signing_client = Client::new_signing(config, mnemonic.parse().unwrap()).unwrap();
    log::info!("Issuing credentials!");
    utils::issue_credential(
        &signing_client.nyxd,
        coin,
        storage.credential_store(),
        recovery_dir,
    )
    .await?;

    let mixnet_client = mixnet::MixnetClientBuilder::new_with_storage(storage)
        .network_details(sandbox_network.clone())
        .enable_credentials_mode()
        .build()
        .await
        .unwrap();

    // Connect using paid bandwidth credential
    let mut client = mixnet_client.connect_to_mixnet().await.unwrap();

    let our_address = client.nym_address();

    // Send a message throughout the mixnet to ourselves
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

    Ok(())
}
