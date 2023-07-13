use nym_sdk::mixnet::{StoragePaths, MixnetClientBuilder, ReconstructedMessage, MixnetClient};
use nym_bin_common::logging::setup_logging;
use std::path::PathBuf;
use nym_sphinx_anonymous_replies::{self, requests::AnonymousSenderTag};
// use rust_cosmos_broadcaster::service::{get_sequence, broadcast}; 
use rust_cosmos_broadcaster::{RequestTypes, SequenceRequestResponse, BroadcastResponse, service::{get_sequence, broadcast}, create_client};

#[tokio::main]
async fn main() {

    // setup_logging();
    let mut client = create_client("/tmp/cosmos-broadcaster-mixnet-server-2".into()).await; 
    let our_address = client.nym_address();
    println!("\nSetup test ---- our client nym address is: {our_address}");
    // TODO create broadcaster in src/service fn, call here & save as var to pass to other fns 

    /*
       TODO 
       * add threads - loop just to check everything works quickly   
     */
    loop {    
        println!("\nWaiting for message");
        // TODO rewrite this to parse any empty SURB data and then parse the actual incoming message 
        // let received = client.wait_for_messages().await;
        
        // handle incoming message - we presume its a reply from the SP 
        // check incoming is empty - SURB requests also send data ( empty vec ) along 
        let mut received: Vec<ReconstructedMessage> = Vec::new(); 

        // get the actual message - discard the empty vec sent along with the SURB request  
        while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            continue;
        } 
        println!("got a response:"); 
        received = new_message;
        println!("{:#?}", &received);
        break  
        }   

        for r in received.iter() {
                // convert incoming vec<u8> -> String 
                let s = String::from_utf8(r.message.clone());
                println!("{:#?}", &s);  
                if s.is_ok() {
                    let p = s.unwrap();
                    println!("{:#?}", &p);  
                    // parse JSON string -> request type & match 
                    let request: RequestTypes = serde_json::from_str(&p).unwrap(); 
                    println!("incoming request: {:#?}", &request);
                    match request {
                        RequestTypes::Sequence(request) => {
                            println!("\nincoming sequence request details:\nvalidator: {},\nsigner address: {}\n", request.validator, request.signer_address); 
                            let sequence: SequenceRequestResponse = get_sequence(request.validator, request.signer_address).await;
                            // print!("debug print -------- {:#?}", sequence); 
                            // println!("debug print SENDER TAG --------- {:#?}", r.sender_tag);
                            if Some(r.sender_tag).is_some() {
                                // println!("debug print ---- sending reply "); 
                                let return_recipient: AnonymousSenderTag = r.sender_tag.unwrap(); 
                                println!("return recipient surb bucket: {}", &return_recipient);
                                // todo actually return sequence serialised as json  
                                client.send_str_reply(return_recipient, &serde_json::to_string(&sequence).unwrap()).await; 
                                // println!("sent reply - sleeping"); 
                                // tokio::time::sleep(Duration::from_secs(25)).await; 
                                // println!("stopped sleep"); 
                            } else {
                                // TODO replace with actual error type to return 
                                println!("no surbs cannot reply an0n") 
                            }
                        },
                        RequestTypes::Broadcast(request) => {
                            println!("\nincoming sequence request details: {}\n", request.base58_tx_bytes);
                            let tx_hash: BroadcastResponse = broadcast(request.base58_tx_bytes).await;
                            if Some(r.sender_tag).is_some() {
                                // println!("debug print ---- sending reply "); 
                                let return_recipient: AnonymousSenderTag = r.sender_tag.unwrap(); 
                                println!("return recipient surb bucket: {}", &return_recipient);
                                // todo actually return sequence serialised as json  
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



