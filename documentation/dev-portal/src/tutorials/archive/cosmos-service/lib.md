# Preparing Your Lib

Now move on to preparing shared data structures and functions in `src/lib.rs`.

These include the request and response types the client and the service will be passing through the mixnet, as well as shared functions such as client creation, and message parsing.

## Dependencies
The dependencies for the shared `lib` file are the following:
```rust
use anyhow::bail;
use cosmrs::AccountId;
use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClient, MixnetClientBuilder, ReconstructedMessage, StoragePaths,
};
use nym_validator_client::nyxd::Coin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod client;
pub mod service;
```

Since this is the file where client creation and message parsing are handled, the various `nym_sdk` imports, as well as `serde`'s (de)serialisation functionality, are required. `PathBuf` is for reading filepaths, `cosmrs` types are required for defining Nyx blockchain accounts, and the `Coin` type from the `nyxd_validator_client` is for our Coin balance request and response. `anyhow` is for easy error handing.

## Constants
Below this are the chain-related `const` variables. These have been hardcoded for this demo.

```rust
pub const DEFAULT_VALIDATOR_RPC: &str = "https://rpc.sandbox.nymtech.net";
pub const DEFAULT_DENOM: &str = "unym";
pub const DEFAULT_PREFIX: &str = "n";
```

These define the RPC endpoint your service will use to interact with the blockchain - in this case the Sandbox testnet - as well as the expected coin denomination, and Bech32-prefix of addresses.

## Shared Data Structures
Define the following structs for our different request and responses that will be serialised and sent through the mixnet between your client and service binaries:

```rust
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BalanceRequest {
    pub validator: String,
    pub account: AccountId,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BalanceResponse {
    pub balance: Coin,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum RequestTypes {
    Balance(BalanceRequest),
}

impl RequestTypes {
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde failure")
    }

    pub fn try_deserialize<M: AsRef<[u8]>>(raw: M) -> anyhow::Result<Self> {
        serde_json::from_slice(raw.as_ref()).map_err(Into::into)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum ResponseTypes {
    Balance(BalanceResponse),
}

impl ResponseTypes {
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde failure")
    }

    pub fn try_deserialize<M: AsRef<[u8]>>(raw: M) -> anyhow::Result<Self> {
        serde_json::from_slice(raw.as_ref()).map_err(Into::into)
    }
}
```

The above data types are pretty straightforward. Even though there are only one instance of a request type (sent from `client` -> mixnet -> `service`) and one of a response type (`service` -> mixnet -> `client`) so far, a pair of enums has been defined to contain additional response and request types that will be added in part 2 of this tutorial, when adding credential functionality.

`BalanceRequest` will be used when requesting the service to query the token balance of the supplied address on the client's behalf. You can see the information that will be returned from the chain to the service, and from the service to the client, in `BalanceResponse`.

Custom serialistion and deserialisation have been implemented for each enum for ease of future modification and testing.

## Shared Functions
Now to define functions shared by the `client` and `service` binaries.

### Client Creation
The following function is called on startup by each binary, with the `config_path` being a filepath for storing client config:

```rust
// create our client with specified path for key storage
pub async fn create_client(config_path: PathBuf) -> MixnetClient {
    let config_dir = config_path;
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .unwrap();

    client.connect_to_mixnet().await.unwrap()
}
```

If no config files exist at the location designated by `config_path` (in this case `/tmp/service`) then the following files are generated:

```sh
service
├── ack_key.pem
├── db.sqlite
├── db.sqlite-shm
├── db.sqlite-wal
├── gateway_details.json
├── gateway_shared.pem
├── persistent_reply_store.sqlite
├── private_encryption.pem
├── private_identity.pem
├── public_encryption.pem
└── public_identity.pem

1 directory, 11 files
```

> If keys and config already exist at this location, re-running this function **will not** overwrite them.

### Listening for & Parsing Incoming messages
Next to define two functions: one for listening _for_ messages from the mixnet (used by `service`), and one for handling a _response_ to a request (used by `client`).

Both functions attempt to deserialise the vec of `ReconstructedMessages` that are reconstructed by the client from delivered Sphinx packets after decryption.

`handle_request` performs one additional function - parsing the `sender_tag` from the incoming reconstructed message. This is the randomised alphanumeric string used to identify a bucket of _SURBs_ (Single Use Reply Blocks) that are sent along with any outgoing message by default. More information about them can be found [here](https://nymtech.net/docs/architecture/traffic-flow.html#private-replies-using-surbs) but all that is necessary to know for now is that these are pre-addressed packets that clients send out with their messages. Any reply to their message that is to be sent back to them back be written to the payload of these packets, but without the replying party being able to see the destination that the reply is being sent to. This allows for services to **anonymously reply to clients without being able to doxx them by knowing their Nym address**.

```rust
pub fn handle_response(message: ReconstructedMessage) -> anyhow::Result<ResponseTypes> {
    ResponseTypes::try_deserialize(message.message)
}

pub fn handle_request(
    message: ReconstructedMessage,
) -> anyhow::Result<(RequestTypes, Option<AnonymousSenderTag>)> {
    let request = RequestTypes::try_deserialize(message.message)?;
    Ok((request, message.sender_tag))
}
```

Before moving on to the `client` and `service` code, one more function is needed. This allows for both binaries to parse empty incoming messages that they might receive. This is necessary as incoming SURBs, as well as requests for more SURBs, contain empty data fields.

```rust
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
```
