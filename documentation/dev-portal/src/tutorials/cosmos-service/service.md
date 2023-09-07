# Preparing Your Service
In `bin/src.rs` define the startup and response logic of the `service`. Client connection / config reading happens as it does in `bin/client.rs`.

## Dependencies
```rust
use chain_query::{
    create_client, handle_request,
    service::{create_broadcaster, get_balance},
    BalanceResponse, RequestTypes, ResponseTypes,
};
use nym_sphinx_anonymous_replies::{self, requests::AnonymousSenderTag};
use nym_bin_common::logging::setup_logging;
use nym_sdk::mixnet::MixnetMessageSender;
```

The imports from `chain_query` are most of the data types and functions defined in the previous sections of this tutorial.

The `AnonymousSenderTag` type is used for SURBs.

## main()
Also using tokio for the async runtime, `main` does the following:
* Create a Nym client with config at `/tmp/service`, or load the existing client from this config directory.
* Create a `broadcaster` - this is used by the service to interact with the blockchain, using the consts defined in `src/lib.rs` as chain config.
* Listen out for incoming messages, and in much the same way as the `client`, handle and match the incoming request.
* Using the `sender_tag`, anonymously reply to the `client` with the response from the blockchain **without having to know the client's Nym address**.

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();
    let mut client = create_client("/tmp/service".into()).await;
    let our_address = client.nym_address();
    println!("\nservice's nym address: {our_address}");
    // the httpclient we will use to broadcast our query to the blockchain
    let broadcaster = create_broadcaster().await?;
    println!("listening for messages, press CTRL-C to exit");

    while let Some(received) = client.wait_for_messages().await {
        for msg in received {
            let request = match handle_request(msg) {
                Ok(request) => request,
                Err(err) => {
                    eprintln!("failed to handle received request: {err}");
                    continue;
                }
            };

            let return_recipient: AnonymousSenderTag = request.1.expect("no sender tag received");
            match request.0 {
                RequestTypes::Balance(request) => {
                    println!("\nincoming balance request for: {}\n", request.account);

                    let balance: BalanceResponse =
                        get_balance(broadcaster.clone(), request.account).await?;

                    let response = ResponseTypes::Balance(balance);

                    println!("response from chain: {:#?}", response);

                    println!("\nreturn recipient surb bucket: {}", &return_recipient);
                    println!("\nsending response to {}", &return_recipient);
                    // send response back to anon requesting client via mixnet
                    let _ = client
                        .send_reply(return_recipient, &serde_json::to_string(&response)?)
                        .await;
                }
            }
        }
    }

    Ok(())
```
