# Preparing Your Service pt2

Now to define the logic of creating the `broadcaster` for interacting with the blockchain, and querying it in `src/service.rs`. 

## Dependencies
The following dependencies are for creating a client to interact with the blockchain, and deal with the returned `Coin` type in `BalanceResponse`. 

```rust
use crate::{BalanceResponse, DEFAULT_DENOM, DEFAULT_VALIDATOR_RPC};
use cosmrs::rpc::HttpClient;
use cosmrs::AccountId;
use nym_validator_client::nyxd::{Coin, CosmWasmClient};
```

## Creating a `broadcaster` 
The `broadcaster` is an `HttpClient` taken from `cosmrs`, created with the `DEFAULT_VALIDATOR_RPC` as its default endpoint. The service will use this to query the Sandbox testnet chain. 

```rust
pub async fn create_broadcaster() -> anyhow::Result<HttpClient> {
    let broadcaster: HttpClient = HttpClient::new(DEFAULT_VALIDATOR_RPC)?;
    Ok(broadcaster)
}
```

## Querying the Chain 
Now to write the logic for querying the chain, using the `broadcaster` created in the previous step to query for the balance of the `account` that the client sent via the mixnet in the `BalanceRequest`. This function returns a `BalanceResponse` containing a `Coin` type, denoting the `balance` and `denom` returned by `get_balance()`. 

```rust
pub async fn get_balance(
    broadcaster: HttpClient,
    account: AccountId,
) -> anyhow::Result<BalanceResponse> {
    let balance = broadcaster
        .get_balance(&account, DEFAULT_DENOM.to_string())
        .await
        .unwrap()
        .unwrap();
    Ok(BalanceResponse {
        balance: Coin {
            amount: balance.amount,
            denom: balance.denom,
        }, 
    })
}
```