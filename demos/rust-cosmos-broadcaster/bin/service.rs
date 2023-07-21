use nym_sphinx_anonymous_replies::{self, requests::AnonymousSenderTag};
use rust_cosmos_broadcaster::{
    create_client, listen_and_parse_request,
    service::{broadcast, create_broadcaster, get_sequence},
    BroadcastResponse, RequestTypes, SequenceRequestResponse,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = create_client("/tmp/cosmos-broadcaster-mixnet-server-3".into()).await;
    let our_address = client.nym_address();
    println!("\nservice's nym address: {our_address}");
    // the httpclient we will use to broadcast our signed tx to the blockchain
    let broadcaster = create_broadcaster().await?;
    println!("listening for messages, press CTRL-C to exit");

    loop {
        // listen out for incoming requests from mixnet, parse and match them
        let request: (RequestTypes, AnonymousSenderTag) =
            listen_and_parse_request(&mut client).await?; 
        // grab sender_tag from parsed request for anonymous replies
        let return_recipient: AnonymousSenderTag = request.1;
        match request.0 {
            RequestTypes::Sequence(request) => {
                println!(
                    "\nincoming sequence request details:\nsigner address: {} \nquerying blockchain on behalf of requesting client",
                    request.signer_address
                );
                // query chain for sequence information on behalf of request sender
                let sequence: SequenceRequestResponse =
                    get_sequence(broadcaster.clone(), request.signer_address)
                        .await?;  
                println!("sequence information returned from chain: account number: {}, sequence:{}, chain id: {} \nsending response to requesting client via mixnet", sequence.account_number, sequence.sequence, sequence.chain_id);
                // send serialised sequence response back to request sender via mixnet
                client
                    .send_str_reply(return_recipient, &serde_json::to_string(&sequence)?)
                    .await;
            }
            RequestTypes::Broadcast(request) => {
                println!(
                    "\nincoming broadcast request: {}\n",
                    request.base58_tx_bytes
                );
                // broadcast the signed tx on behalf of request sender
                let tx_hash: BroadcastResponse =
                    broadcast(request.base58_tx_bytes, broadcaster.clone())
                    .await?;
                println!("return recipient surb bucket: {}", &return_recipient);
                // send response to tx (transaction hash) back to request sender via mixnet
                client
                    .send_str_reply(return_recipient, &serde_json::to_string(&tx_hash)?)
                    .await;
            }
        }
    }
}
