use nym_sdk::mixnet::{Recipient, MixnetClient, StoragePaths, MixnetClientBuilder};
use nym_validator_client::nyxd::AccountId;
use nym_bin_common::logging::setup_logging;
use std::path::PathBuf;
mod commands; 
use serde::{Deserialize, Serialize};
use cosmrs::rpc::{Id};
use nym_sphinx_anonymous_replies::{self, requests::RepliableMessage}; 


#[derive(Debug, Deserialize, Serialize)]
struct SequenceRequest {
    validator: String, 
    signer_address: AccountId, 
    // request_type: String
}
#[derive(Deserialize, Serialize)]
struct SequenceResponse {
    sequence_response: u8, 
    chain_id: Id
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum RequestTypes {
    Sequence(SequenceRequest), 
    // Broadcast(BroadcastRequest)
}
enum ResponseTypes {
    Sequence(SequenceResponse), 
    // Broadcast(BroadcastResponse)
}

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

    println!("\nWaiting for message");
    if let Some(received) = client.wait_for_messages().await {
         
        for r in &received {
            let s = String::from_utf8(r.message.clone()); 
            if s.is_ok() {
                let p = s.unwrap(); 
                let request: RequestTypes = serde_json::from_str(&p).unwrap(); 
                println!("incoming request: {:#?}", &request);
                match request {
                    RequestTypes::Sequence(SequenceRequest) => {
                        println!("matched!")
                        // TODO pass to commands::fn 
                    }, 
                    // RequestTypes::Broadcast(BroadcastRequest) // TODO 
                    _ => {
                        println!(" (x_x) ")
                    }
                } 
            } 

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
