# Nym API Setup

[//]: # (> The Nym API binary was built in the [building nym]&#40;../binaries/building-nym.md&#41; section. If you haven't yet built Nym and want to run the code, go there first. You can build just the API with `cargo build --release --bin nym-api`.)
[//]: # ()

> The `nym-api` binary should be coming out in the next release - we're releasing this document beforehand so that Validators have information as soon as possible and get an idea of what to expect. This doc will be expanded over time as we release the API binary itself as well as start enabling functionality.  

## What is the Nym API?
The Nym API is a binary that will be operated by some or all of the Nyx Blockchain Validator set. THie binary can be run in several different modes, and has two main bits of functionality: 
* network monitoring (calculating the routing score of Mixnet nodes) 
* generation and validation of [zk-Nyms](), our implementation of the Coconut Selective Disclosure Credential Scheme.

This is important for both the proper decentralisation of the network uptime calculation and, more pressingly, enabling the NymVPN to utilise privacy preserving payments. 

The process of enabling these different aspects of the system will take time. Operators of the Nym API binary for the moment will only have to run the binary in a minimal 'caching' mode in order to get used to maintaining an additional process running alongside their Validator. 

### Rewards 
Operators of Nym API instances will be rewarded for performing the extra work of taking part in credential generation. These rewards will be calculated **separately** from rewards for block production. 

Rewards for credential signing will be calculated hourly, with API operators receiving a proportional amount of the reward pool (333NYM per hour / 237600NYM per month) according to the % of credentials they have signed. 

### (Coming Soon) Machine Specs 
We are working on load testing currently in order to get good specs for a Validator + Nym API setup. Bear in mind that credential signing is primarily CPU-bound. 

### (Coming Soon) Credential Generation
Validators that take part in the DKG ceremony (more details on this soon) will become part of the quorum generating and verifying zk-Nym credentials. These will initially be used for private proof of payment for NymVPN (see our blogposts [here](https://blog.nymtech.net/nymvpn-an-invitation-for-privacy-experts-and-enthusiasts-63644139d09d) and [here](https://blog.nymtech.net/zk-nyms-are-here-a-major-milestone-towards-a-market-ready-mixnet-a3470c9ab10a) for more on this), and in the future will be expanded into more general usecases such as [offline ecash](https://arxiv.org/abs/2303.08221). 

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

### Running your Nym API Instance 
The API binary currently defaults to running in caching mode. 

By default the API will be trying to query a running `nyxd` process (either a validator or RPC node) on `localhost:26657`. This value can be modified either via the `--nyxd-validator ` flag on `run`, or changing the value of `local_validator` in the config file found by default in `$HOME/.nym/nym-api/<ID>/config/config.toml`.  

You can run your API with: 

```
./nym-api run
```

## Automation 
TODO 