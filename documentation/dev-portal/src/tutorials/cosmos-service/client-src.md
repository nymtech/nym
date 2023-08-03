# Preparing Your Client pt2

Open `src/client.rs`. This is where the logic of the command from the `match` statement in `bin/client.rs` is defined.

# Dependencies
```rust
use crate::{handle_response, wait_for_non_empty_message, RequestTypes, DEFAULT_VALIDATOR_RPC};
use cosmrs::AccountId;
use nym_sdk::mixnet::MixnetClient;
use nym_sphinx_addressing::clients::Recipient;
use nym_validator_client::nyxd::Coin
```

# Querying via the Mixnet
The following is used to construct a `BalanceRequest`, send this to the supplied `service` address, and then handle the response, matching it to a `ResponseType`, in this case the only expected response, a `BalanceResponse`.

The actual sending of the request is performed by `client.send_bytes`: sending the serialised `BalanceRequest` to the supplied Nym address (the `Recipient` imported from the `nym_sphinx_addressing` crate). It is sending the default number of SURBs along with the message, defined [here](https://github.com/nymtech/nym/blob/develop/sdk/rust/nym-sdk/src/mixnet/client.rs#L34).

```rust
pub async fn query_balance(
    account: AccountId,
    client: &mut MixnetClient,
    sp_address: Recipient,
) -> anyhow::Result<Coin> {
    // construct balance request
    let message = RequestTypes::Balance(crate::BalanceRequest {
        validator: DEFAULT_VALIDATOR_RPC.to_owned(), // rpc endpoint for broadcaster to use
        account,
    });

    // send serialised request to service via mixnet
    client
        .send_bytes(sp_address, message.serialize(), Default::default())
        .await;

    let received = wait_for_non_empty_message(client).await?;

    // listen for response from service
    let sp_response = handle_response(received)?;

    // match JSON -> ResponseType
    let res = match sp_response {
        crate::ResponseTypes::Balance(response) => {
            println!("{:#?}", response);
            response.balance
        }
    };

    Ok(res)
}
```

That is all of the client code written: now to move on to the `service` that will be interacting with the blockchain on behalf of the `client`.
