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

Since this is the file where client creation and message parsing are handled, the various `nym_sdk` imports, as well as `serde`'s (de)serialisation functionality, are required. This file also imports the `client` and `service` specific logic not found in `bin/`, as well as `PathBuf` to read filepaths, and `cosmrs` types for defining Nyx blockchain accounts.

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
    pub chain_id: tendermint::chain::Id,
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

TODO explainer of each

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
TODO smoosh them into one function with the return type being an `Option<sender_tag>` instead

Next to define two functions: one for listening _for_ messages from the mixnet (used by our `service`), and one for listening out for a _reply_ after sending a message to another Nym client (in this case, when sending a message from the `client` to the `service`).
