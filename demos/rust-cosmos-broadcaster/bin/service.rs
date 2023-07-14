use nym_sdk::mixnet::ReconstructedMessage;
use nym_bin_common::logging::setup_logging;
use nym_sphinx_anonymous_replies::{self, requests::AnonymousSenderTag};
use rust_cosmos_broadcaster::{RequestTypes, SequenceRequestResponse, BroadcastResponse, service::{get_sequence, broadcast, create_broadcaster}, create_client};

#[tokio::main]
async fn main() {
    // setup_logging();
    let mut client = create_client("/tmp/cosmos-broadcaster-mixnet-server-2".into()).await; 
    let our_address = client.nym_address();
    println!("\nservice's nym address: {our_address}");
    // the httpclient we will use to broadcast our signed tx to the Nyx blockchain  
    let broadcaster = create_broadcaster().await; 

    loop {    
        println!("\nWaiting for new message");
        // check incoming is empty - SURB requests also send data ( empty vec ) along 
        let mut received: Vec<ReconstructedMessage> = Vec::new(); 
        // get the actual message - discard the empty vec sent along with the SURB request  
        while let Some(new_message) = client.wait_for_messages().await {
            if new_message.is_empty() {
                continue;
            } 
            received = new_message;
            break  
        }   

        for r in received.iter() {
                // convert incoming vec<u8> -> String 
                let s = String::from_utf8(r.message.clone());
                // println!("{:#?}", &s);  
                if s.is_ok() {
                    let p = s.unwrap();
                    // println!("{:#?}", &p);  
                    // parse JSON string -> request type & match 
                    let request: RequestTypes = serde_json::from_str(&p).unwrap(); 
                    // println!("\nincoming request: {:#?}", &request);
                    match request {
                        RequestTypes::Sequence(request) => {
                            println!("\nincoming sequence request details:\nsigner address: {}", request.signer_address); 
                            let sequence: SequenceRequestResponse = get_sequence(broadcaster.clone(), request.signer_address).await;
                            if Some(r.sender_tag).is_some() {
                                let return_recipient: AnonymousSenderTag = r.sender_tag.unwrap();
                                println!("replying to sender with sequence response from Nyx chain"); 
                                println!("return recipient surb bucket: {}", &return_recipient);
                                client.send_str_reply(return_recipient, &serde_json::to_string(&sequence).unwrap()).await; 
                            } else {
                                // TODO replace with actual error type to return 
                                println!("no surbs cannot reply an0n") 
                            }
                        },
                        RequestTypes::Broadcast(request) => {
                            println!("\nincoming broadcast request: {}\n", request.base58_tx_bytes);
                            let tx_hash: BroadcastResponse = broadcast(request.base58_tx_bytes, broadcaster.clone()).await;
                            if Some(r.sender_tag).is_some() {
                                let return_recipient: AnonymousSenderTag = r.sender_tag.unwrap(); 
                                println!("return recipient surb bucket: {}", &return_recipient);
                                client.send_str_reply(return_recipient, &serde_json::to_string(&tx_hash).unwrap()).await; 
                            } else {
                                // TODO replace with actual error type to return 
                                println!("no surbs cannot reply an0n") 
                            }

                        }  
                    } 
                } 
        }   
    }
}



