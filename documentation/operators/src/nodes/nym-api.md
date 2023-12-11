# Nym API Setup

> The Nym API binary was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first. You can build just the API with `cargo build --release --bin nym-api`.

## What is the Nym API?
The Nym API is a binary that will be operated by some or all of the Nyx Blockchain Validator set. This is important for the proper decentralisation of the network monitoring (calculating the routing score of Mixnet nodes), as well as the creation of credentials via DKG (Distributed Key Generation). 

> For the moment much of this functionality is not supported - functionality will be slowly enabled over time. For the moment the API will be running in caching mode in order to allow runners to get used to maintaining the extra process alongside their Validator.  

### Decentralised Network Monitoring (coming soon) 
**TODO** 

### Credential Generation (coming soon)
Validators that take part in the DKG ceremony will become part of the quorum generating [zk-Nyms](), which will initially be used for private proof of payment for NymVPN. 

TODO reward formula 

## Current version
```
<!-- cmdrun ../../../../target/release/nym-api --version | grep "Build Version" | cut -b 21-26  -->
```

## Setup and Usage
### Viewing command help
You can check that your binary is properly compiled with:

```
./nym-api --help
```

Which should return a list of all available commands.

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-api init --help -->
```
~~~

You can also check the various arguments required for individual commands with:

```
./nym-api <COMMAND> --help
```

### Initialising your Nym API Instance 
Initialise your API instance with: 

```
./nym-api init
```

You can optionally pass a local identifier for this instance with the `--instance` flag. Otherwise the ID of your instance defaults to `default`. 

**TODO add example output - don't run with `cmdrun` just yet**

### Running your Nym API Instance 
The API binary currently defaults to running in caching mode. 

By default the API will be trying to query a running `nyxd` process (either a validator or RPC node) on `localhost:26657`. This value can be modified either via the `--nyxd-validator ` flag on `run`, or changing the value of `local_validator` in the config file found by default in `$HOME/.nym/nym-api/<ID>/config/config.toml`.  

You can run your API with: 

```
./nym-api run
```

## Automation 
TODO 