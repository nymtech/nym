# Preparing Your Lib

Now move on to preparing shared data structures and functions in `src/lib.rs`.

These include the request and response types the client and the service will be passing through the mixnet, as well as shared functions such as client creation, and message parsing.

## Dependencies
The dependecies for the shared `lib` file are the following:
```rust
use cosmrs::{tendermint, AccountId};
use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClient, MixnetClientBuilder, ReconstructedMessage, StoragePaths,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub mod client;
pub mod service;
```

Since this is the file where client creation and message parsing are handled, the various `nym_sdk` imports, as well as `serde`'s (de)serialisation functionality, are required. `PathBuf` is for reading filepaths, and `cosmrs` types are required for defining Nyx blockchain accounts.

## Constants
Below this are the chain-related `const` variables. These have been hardcoded for this demo.

```rust
pub const DEFAULT_VALIDATOR_RPC: &str = "https://sandbox-validator1.nymtech.net";
pub const DEFAULT_DENOM: &str = "unym";
pub const DEFAULT_PREFIX: &str = "n";
```

These define the RPC endpoint your service will use to interact with the blockchain - in this case the Sandbox testnet - as well as the coin denomination and Bech32-prefix of the blockchain accounts.

## Shared Data Structures
Define the following structs for our different request and responses that will be serialised and sent through the mixnet between your client and service binaries:

```rust
#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequest {
    pub validator: String,
    pub signer_address: AccountId,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SequenceRequestResponse {
    pub account_number: u64,
    pub sequence: u64,
    pub chain_id: [tendermint](tendermint)::chain::Id,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequestTypes {
    Sequence(SequenceRequest),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResponseTypes {
    Sequence(SequenceRequestResponse),
}
```

The above data types are pretty straightforward. Even though there are only one instance of a request type (sent from client to service) and one of a response type (service -> client) so far, a pair of enums has been defined to contain additional response or request types that will be added in the future.

`SequenceRequest` will be used when requesting the service to query the chain on the client's behalf for an address' sequence information (used for offline signing). You can see the information that will be returned from the chain to the service, and from the service to the client, in `SequenceRequestResponse`.

> Although `SequenceResponse` would have been more succinct, there is already a `cosmrs` type with this name. As such the response type was given a different name to avoid confusion.

## Shared Functions
Now to define functions shared by the `client` and `service` binaries.

### Client Creation
The following function is called on startup by each binary, with the `config_path` being a filepath for storing client config.

```rust
// create our client with specified path for key storage
pub async fn create_client(config_path: PathBuf) -> MixnetClient {
    let config_dir = config_path;
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .await
        .unwrap();

    client.connect_to_mixnet().await.unwrap()
}
```

> If keys and config already exist at this location, re-running this function **will not** overwrite them.

### Parsing Incoming messages
Next to define two functions: one for listening _for_ messages from the mixnet (used by our `service`), and one for listening out for a _reply_ after sending a message to another Nym client (in this case, when sending a message from the `client` to the `service`).

```rust
pub async fn listen_and_parse_response(client: &mut MixnetClient) -> anyhow::Result<ResponseTypes> {
    let mut message: Vec<ReconstructedMessage> = Vec::new();

    // get the actual message - discard the empty vec sent along with the SURB topup request
    while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            println!("got a request for more SURBs from service - sending additional SURBs to sender");
            continue;
        }
        message = new_message;
        break;
    }

    // parse vec<u8> -> JSON String
    let mut parsed = String::new();
    if let Some(r) = message.iter().next() {
        parsed = String::from_utf8(r.message.clone())?;
    }
    let sp_response: crate::ResponseTypes = serde_json::from_str(&parsed)?;
    Ok(sp_response)
}

pub async fn listen_and_parse_request(
    client: &mut MixnetClient,
) -> anyhow::Result<(RequestTypes, AnonymousSenderTag)> {
    let mut message: Vec<ReconstructedMessage> = Vec::new();

    while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            println!("got empty vec - probably the SURBs sent along with the request");
            continue;
        }
        message = new_message;
        break;
    }

    // parse vec<u8> -> JSON String
    let mut parsed = String::new();
    if let Some(r) = message.iter().next() {
        parsed = String::from_utf8(r.message.clone())?;
    }
    let client_request: crate::RequestTypes = serde_json::from_str(&parsed)?;

    // get the sender_tag for anon reply
    let return_recipient = message[0].sender_tag.unwrap();

    Ok((client_request, return_recipient))
}
```

Aside from the return types that each function has, the main difference is that any incoming requests to the `service` will also have SURB packets attached to it.

< smol explanation of SURBs + link >

As such, this function returns a tuple containing the `RequestType` _and_ the `sender_tag` used by the `service` to identify which bucket of pre-addressed replies it will use to respond to a request.

< note concerning parsing out the empty vecs >
