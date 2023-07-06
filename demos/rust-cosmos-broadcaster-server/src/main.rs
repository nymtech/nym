use nym_sdk::mixnet::{Recipient, MixnetClient, StoragePaths, MixnetClientBuilder};
use nym_validator_client::nyxd::AccountId;
use nym_bin_common::logging::setup_logging;
use std::path::PathBuf;
mod commands; 

#[tokio::main]
async fn main() {
    setup_logging();

    let config_dir = PathBuf::from("/tmp/cosmos-broadcaster-mixnet-server");
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .await
        .unwrap();
    let mut client = client.connect_to_mixnet().await.unwrap();
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        // parse traffic based on... something ... and match {} to functions in commands 
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
            /*
            deserialise json 
            check fn struct match 
            match{} and pass to function 
            */
        }
    }

    // TODO V2 - multithreading! 
    client.disconnect().await;
}



/* 
code for sequence and chain 
    // possibly remote client that doesn't do ANY signing
    // (only broadcasts + queries for sequence numbers)
    let broadcaster = HttpClient::new(validator).unwrap();

    // get signer information
    let sequence_response = broadcaster.get_sequence(&signer_address).await.unwrap();
    let chain_id = broadcaster.get_chain_id().await.unwrap();
    -> pass back chain_id and sequence_response to client side 

code for broadcast 
    // decode the base58 tx to vec<u8>

    // broadcast the tx
    let res = rpc::Client::broadcast_tx_commit(&broadcaster, tx_bytes.into())
    .await
    .unwrap();

    // send res back via SURBs 
 */
