# Message Helpers

## Handling incoming messages
As seen in the [Chain querier tutorial](https://github.com/nymtech/developer-tutorials/blob/0130ee5a61cd6801bdcfc84608b2a520b5392714/rust/chain-query-service/) when listening out for a response to a sent message (e.g. if you have sent a request to a service, and are awaiting the response) you will want to await [non-empty messages (more info on this here)](troubleshooting.md#client-receives-empty-messages-when-listening-for-response). This can be done with something like the helper functions [here](https://github.com/nymtech/developer-tutorials/blob/0130ee5a61cd6801bdcfc84608b2a520b5392714/rust/chain-query-service/src/lib.rs#L71). 

## Iterating over incoming messages
It is recommended to use `nym_client.next().await` over `nym_client.wait_for_messages().await` as the latter will return one message at a time which will probably be easier to deal with. See the [parallel send and receive example](https://github.com/nymtech/nym/blob/2993e85c7a17bd5b68171751a48b731b2394ee03/sdk/rust/nym-sdk/examples/parallel_sending_and_receiving.rs#L23-L25) for an example. 

## Remember to disconnect your client
You should always **manually disconnect your client** with `client.disconnect().await` as seen in the code examples. This is important as your client is writing to a local DB and dealing with SURB storage. 
