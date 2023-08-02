# Preparing Your Client

Start by creating the startup logic of your `client` - creating a Nym client and connecting to the mixnet (or just connecting if your client has been started before and config already exists for it), and defining commands.

Start in `bin/client.rs`.

## Dependencies
Import the following dependencies:
```
use clap::{Args, Parser, Subcommand};
use nym_sdk::mixnet::Recipient;
use nym_validator_client::nyxd::AccountId;
use nym_cosmos_service::create_client;
use nym_bin_common::logging::setup_logging;
```

`clap` is used so different commands can be passed to the `client` (even though we're only defining one function in this first part of the tutorial, more will be added in subsequent chapters). `nym_sdk::mixnet::Recipient` is the type used to define the recipient of a mixnet message, `nym_bin_common::logging::setup_logging` is the logging setup for `client`'s Nym client, and `nym_cosmos_service::create_client` imports the `create_client` function created on the previous page.

## CLI Command with Clap

< simple account query first - then maybe a contract query >

##
