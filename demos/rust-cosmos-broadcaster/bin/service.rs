use nym_sphinx_anonymous_replies::{self, requests::AnonymousSenderTag};
use rust_cosmos_broadcaster::{
    create_client, listen_and_parse_request,
    service::{broadcast, create_broadcaster, get_sequence},
    BroadcastResponse, RequestTypes, SequenceRequestResponse,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // setup_logging();
    let mut client = create_client("/tmp/cosmos-broadcaster-mixnet-server-3".into()).await;
    let our_address = client.nym_address();
    println!("\nservice's nym address: {our_address}");
    // the httpclient we will use to broadcast our signed tx to the Nyx blockchain
    let broadcaster = create_broadcaster().await;

    loop {
        let request: (RequestTypes, AnonymousSenderTag) =
            listen_and_parse_request(&mut client).await;
        let return_recipient: AnonymousSenderTag = request.1;
        match request.0 {
            RequestTypes::Sequence(request) => {
                println!(
                    "\nincoming sequence request details:\nsigner address: {}",
                    request.signer_address
                );
                let sequence: SequenceRequestResponse =
                    get_sequence(broadcaster.clone(), request.signer_address)
                        .await
                        .unwrap();
                client
                    .send_str_reply(return_recipient, &serde_json::to_string(&sequence).unwrap())
                    .await;
            }
            RequestTypes::Broadcast(request) => {
                println!(
                    "\nincoming broadcast request: {}\n",
                    request.base58_tx_bytes
                );
                let tx_hash: BroadcastResponse =
                    broadcast(request.base58_tx_bytes, broadcaster.clone())
                        .await
                        .unwrap();
                println!("return recipient surb bucket: {}", &return_recipient);
                client
                    .send_str_reply(return_recipient, &serde_json::to_string(&tx_hash).unwrap())
                    .await;
            }
        }
    }
}
