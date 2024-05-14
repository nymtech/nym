# Message Helpers

## Handling incoming messages
When listening out for a response to a sent message (e.g. if you have sent a request to a service, and are awaiting the response) you will want to await non-empty messages [(if you don't know why, read the info on this here)](troubleshooting.md#client-receives-empty-messages-when-listening-for-response). This can be done with something like the helper functions [here](https://github.com/nymtech/developer-tutorials/blob/0130ee5a61cd6801bdcfc84608b2a520b5392714/rust/chain-query-service/src/lib.rs#L71): 

```rust
use nym_sdk::mixnet::ReconstructedMessage; 

pub async fn wait_for_non_empty_message(
    client: &mut MixnetClient,
) -> anyhow::Result<ReconstructedMessage> {
    while let Some(mut new_message) = client.wait_for_messages().await {
        if !new_message.is_empty() {
            return Ok(new_message.pop().unwrap());
        }
    }
    
    bail!("did not receive any non-empty message")
}

pub fn handle_response(message: ReconstructedMessage) -> anyhow::Result<ResponseTypes> {
    ResponseTypes::try_deserialize(message.message)
}

// Note here that the only difference between handling a request and a response
// is that a request will have a sender_tag to parse. 
// 
// This is used for anonymous replies with SURBs.  
pub fn handle_request(
    message: ReconstructedMessage,
) -> anyhow::Result<(RequestTypes, Option<AnonymousSenderTag>)> {
    let request = RequestTypes::try_deserialize(message.message)?;
    Ok((request, message.sender_tag))
}
```

The above helper functions are used as such by the client in tutorial example: it sends a message to the service (what the message is isn't important - just that your client has sent a message _somewhere_ and you are awaiting a response), waits for a _non_empty_ message, then handles it (then logs it - but you can do whatever you want, parse it, etc): 

```rust
// [snip]

// Send serialised request to service via mixnet what is await-ed here is 
// placing the message in the client's message queue, NOT the sending itself.
let _ = client
    .send_message(sp_address, message.serialize(), Default::default())
    .await;

// Await a non-empty message 
let received = wait_for_non_empty_message(client).await?;

// Handle the response received (the non-empty message awaited above) 
let sp_response = handle_response(received)?;

// Match JSON -> ResponseType
let res = match sp_response {
    crate::ResponseTypes::Balance(response) => {
        println!("{:#?}", response);
        response.balance
    }
};

// [snip]
```
([repo code on Github here](https://github.com/nymtech/developer-tutorials/blob/0130ee5a61cd6801bdcfc84608b2a520b5392714/rust/chain-query-service/src/client.rs#L19))

## Iterating over incoming messages
It is recommended to use `nym_client.next().await` over `nym_client.wait_for_messages().await` as the latter will return one message at a time which will probably be easier to deal with. See the [parallel send and receive example](https://github.com/nymtech/nym/blob/2993e85c7a17bd5b68171751a48b731b2394ee03/sdk/rust/nym-sdk/examples/parallel_sending_and_receiving.rs#L23-L25) for an example. 

## Remember to disconnect your client
You should always **manually disconnect your client** with `client.disconnect().await` as seen in the code examples. This is important as your client is writing to a local DB and dealing with SURB storage. 
