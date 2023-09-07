# Preparing Your Client

Start by creating the startup logic of your `client` in `bin/client.rs` - creating a Nym client and connecting to the mixnet (or just connecting if your client has been started before and config already exists for it), and defining and running commands.

## Dependencies
Import the following dependencies:
```rust
use clap::{Args, Parser, Subcommand};
use chain_query::{client::query_balance, create_client};
use nym_sdk::mixnet::Recipient;
use nym_validator_client::nyxd::AccountId;
use nym_bin_common::logging::setup_logging;
```

`clap` is used so different commands can be passed to the `client` (even though we're only defining one function in this first part of the tutorial, more will be added in subsequent chapters). `nym_sdk::mixnet::Recipient` is the type used to define the recipient of a mixnet message, `nym_bin_common::logging::setup_logging` is the logging setup for `client`'s Nym client, and `chain_query` imports the `create_client` and `query_balance` functions created on the previous page.

## CLI Command with Clap
The following simply defines the commands that the client can perform. For the  moment, there is only one: the `query_balance` function created in the previous section.

As with the data structures, this structure is being used for ease of adding future commands in subsequent tutorials.

```rust
#[derive(Debug, Parser)]
#[clap(name = "rust sdk demo - chain query service")]
#[clap(about = "query the sandbox testnet blockchain via the mixnet... part 2 coming soon")]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    QueryBalance(QueryBalance),
}

#[derive(Debug, Args)]
struct QueryBalance {
    /// the account we want to query
    account: AccountId,
    /// the address of the broadcaster service - this submits txs and queries the chain on our behalf
    sp_address: String,
}
```

## `main()`
This is the root logic of the `client`. Using `[tokio](https://tokio.rs/)` for the async runtime, this function performs the following functions:
* If not already existing, create a Nym client with config at `/tmp/client`. Otherwise load the already existing client from this config.
* Matche the command from the CLI - in this instance, the `QueryBalance` function which will be defined in the next section. This creates a `BalanceRequest` and sends this to the `service`, before returning the response back to the main thread and print this to the console.
* Perform a proper shutdown of the Nym client.

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();
    let cli = Cli::parse();
    let mut client = create_client("/tmp/client2".into()).await;
    let our_address = client.nym_address();
    println!("\nclient's nym address: {our_address}");

    match cli.command {
        Some(Commands::QueryBalance(QueryBalance {
            account,
            sp_address,
        })) => {
            println!("\nsending bank balance request to service via mixnet");
            let sp_address = Recipient::try_from_base58_string(sp_address).unwrap();
            let returned_balance = query_balance(account, &mut client, sp_address).await?;
            println!("\nreturned balance is: {}", returned_balance);
        }
        None => {
            println!("\nno command specified - nothing to do")
        }
    }
    println!("\ndisconnecting client");
    client.disconnect().await;
    println!("client disconnected");
    Ok(())
}
```
