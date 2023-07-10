use nym_sdk::mixnet::{Recipient, MixnetClient, StoragePaths, MixnetClientBuilder};
use nym_validator_client::nyxd::AccountId;
use nym_bin_common::logging::setup_logging;
use std::path::PathBuf;
mod commands; 
use serde::{Deserialize, Serialize};
use cosmrs::{rpc::{Id}};
use nym_sphinx_anonymous_replies::{self, requests::RepliableMessage};
use crate::commands::reqres;
// pull in  ::reply::{MixnetAddress, MixnetMessage} 

#[tokio::main]
async fn main() {
    setup_logging();
    // TODO put client creation in own fn 
    let config_dir = PathBuf::from("/tmp/cosmos-broadcaster-mixnet-server-2");
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .await
        .unwrap();
    let mut client = client.connect_to_mixnet().await.unwrap();
    let our_address = client.nym_address();
    println!("\nOur client nym address is: {our_address}");

    /*
       TODO 
       * make process just wait for another message instead of end after single match() 
       * add threads!  
     */
    println!("\nWaiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in &received {
            let s = String::from_utf8(r.message.clone()); 
            if s.is_ok() {
                let p = s.unwrap(); 
                let request: reqres::RequestTypes = serde_json::from_str(&p).unwrap(); 
                println!("incoming request: {:#?}", &request);
                match request {
                    reqres::RequestTypes::Sequence(request) => {
                        println!("\nincoming sequence request details:\nvalidator: {},\nsigner address: {}\n", request.validator, request.signer_address); 
                        let sequence: reqres::SequenceRequestResponse = commands::commands::get_sequence(request.validator, request.signer_address).await;
                        print!("debug print -------- {:#?}", sequence); 
                        
                        // send back to sender via SURBS... 
                        // TODO how to properly parse to AnonymousSenderTag? 
                        // TODO make a rust sdk example of replying via SURBs
                        println!("debug print SENDER TAG --------- {:#?}", r.sender_tag);

                        if Some(r.sender_tag).is_some() {
                            // TODO construct reply 
                            let return_recipient = MixnetAddress::from(r.sender_tag); 
                            // send back thru mixnet
                            client.send_str(return_recipient, &serde_json::to_string(&sequence).unwrap()).await; 
                        } else {
                        //     // TODO replace with actual error type to return 
                            println!("no surbs cannot reply an0n") 
                        }
                    },
                    reqres::RequestTypes::Broadcast(BroadcastRequest) => {
                        todo!()
                    }  
                } 
            } 
        }
    }

    client.disconnect().await;
}



