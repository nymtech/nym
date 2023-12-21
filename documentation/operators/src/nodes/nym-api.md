# Nym API Setup

[//]: # (> The nym-api binary was built in the [building nym]&#40;../binaries/building-nym.md&#41; section. If you haven't yet built Nym and want to run the code, go there first. You can build just the API with `cargo build --release --bin nym-api`.)
[//]: # ()

> The `nym-api` binary should be coming out in the next release - we're releasing this document beforehand so that validators have information as soon as possible and get an idea of what to expect. This doc will be expanded over time as we release the API binary itself as well as start enabling functionality.

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

## What is the Nym API?
The Nym API is a binary that will be operated by the Nyx validator set. This binary can be run in several different modes, and has two main bits of functionality:
* network monitoring (calculating the routing score of Mixnet nodes)
* generation and validation of [zk-Nyms](https://blog.nymtech.net/zk-nyms-are-here-a-major-milestone-towards-a-market-ready-mixnet-a3470c9ab10a), our implementation of the Coconut Selective Disclosure Credential Scheme.

This is important for both the proper decentralisation of the network uptime calculation and, more pressingly, enabling the NymVPN to utilise privacy preserving payments.

The process of enabling these different aspects of the system will take time. At the moment, Nym API operators will only have to run the binary in a minimal 'caching' mode in order to get used to maintaining an additional process running alongside a full node.

```admonish warning
It is highly recommended to run `nym-api` alongside a full node since you will be exposing HTTP port(s) to the Internet. We also observed degradation in p2p and block signing operations when `nym-api` was run alongside a signing validator.

```

### Rewards
Operators of Nym API will be rewarded for performing the extra work of taking part in credential generation. These rewards will be calculated **separately** from rewards for block production.

Rewards for credential signing will be calculated hourly, with API operators receiving a proportional amount of the reward pool (333NYM per hour / 237,600 NYM per month), proportional to the percentage of credentials they have signed.

### (Coming Soon) Machine Specs
We are working on load testing currently in order to get good specs for a full node + Nym API setup. Bear in mind that credential signing is primarily CPU-bound.

### (Coming Soon) Credential Generation
Validators that take part in the DKG ceremony (more details on this soon) will become part of the quorum generating and verifying zk-Nym credentials. These will initially be used for private proof of payment for NymVPN (see our blogposts [here](https://blog.nymtech.net/nymvpn-an-invitation-for-privacy-experts-and-enthusiasts-63644139d09d) and [here](https://blog.nymtech.net/zk-nyms-are-here-a-major-milestone-towards-a-market-ready-mixnet-a3470c9ab10a) for more on this), and in the future will be expanded into more general usecases such as [offline e-cash](https://arxiv.org/abs/2303.08221).

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
<!-- cmdrun ../../../../target/release/nym-api --help -->
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

You can optionally pass a local identifier for this instance with the `--id` flag. Otherwise the ID of your instance defaults to `default`.

### Running your Nym API Instance
The API binary currently defaults to running in caching mode. You can run your API with:

```
./nym-api run
```

By default the API will be trying to query a running `nyxd` process (either a validator or RPC node) on `localhost:26657`. This value can be modified either via the `--nyxd-validator ` flag on `run`:

```
./nym-api run --nyxd-validator https://rpc.nymtech.net:443
```

> You can also change the value of `local_validator` in the config file found by default in `$HOME/.nym/nym-api/<ID>/config/config.toml`.

This process is quite noisy, but informative:

~~~admonish example collapsible=true title="Console output"
```
Starting nym api...
 2023-12-12T14:29:55.800Z INFO  rocket::launch > 🔧 Configured for release.
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > address: 127.0.0.1
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > port: 8000
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > workers: 4
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > max blocking threads: 512
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > ident: Rocket
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > IP header: X-Real-IP
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > limits: bytes = 8KiB, data-form = 2MiB, file = 1MiB, form = 32KiB, json = 1MiB, msgpack = 1MiB, string = 8KiB
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > temp dir: /tmp
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > http/2: true
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > keep-alive: 5s
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > tls: disabled
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > shutdown: ctrlc = true, force = true, signals = [SIGTERM], grace = 2s, mercy = 3s
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > log level: critical
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > cli colors: true
 2023-12-12T14:29:55.800Z INFO  rocket::launch    > 📬 Routes:
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_registered_names) GET /v1/names
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_mixnodes) GET /v1/mixnodes
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_gateways) GET /v1/gateways
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_services) GET /v1/services
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /v1/openapi.json
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_full_circulating_supply) GET /v1/circulating-supply
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_current_epoch) GET /v1/epoch/current
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_active_set) GET /v1/mixnodes/active
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_mixnodes_detailed) GET /v1/mixnodes/detailed
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_rewarded_set) GET /v1/mixnodes/rewarded
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_gateways_described) GET /v1/gateways/described
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_interval_reward_params) GET /v1/epoch/reward_params
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_blacklisted_mixnodes) GET /v1/mixnodes/blacklisted
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_blacklisted_gateways) GET /v1/gateways/blacklisted
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_total_supply) GET /v1/circulating-supply/total-supply-value
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_circulating_supply) GET /v1/circulating-supply/circulating-supply-value
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_active_set_detailed) GET /v1/mixnodes/active/detailed
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_rewarded_set_detailed) GET /v1/mixnodes/rewarded/detailed
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /cors/<status>
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/index.css
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/index.html
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/swagger-ui.css
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/oauth2-redirect.html
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/swagger-ui-bundle.js
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/swagger-ui-config.json
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/swagger-initializer.js
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > GET /swagger/swagger-ui-standalone-preset.js
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_mixnodes_detailed) GET /v1/status/mixnodes/detailed
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_mixnode_inclusion_probabilities) GET /v1/status/mixnodes/inclusion_probability
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_mixnode_status) GET /v1/status/mixnode/<mix_id>/status
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_active_set_detailed) GET /v1/status/mixnodes/active/detailed
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_rewarded_set_detailed) GET /v1/status/mixnodes/rewarded/detailed
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_mixnode_stake_saturation) GET /v1/status/mixnode/<mix_id>/stake-saturation
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (get_mixnode_inclusion_probability) GET /v1/status/mixnode/<mix_id>/inclusion-probability
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (network_details) GET /v1/network/details
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (nym_contracts) GET /v1/network/nym-contracts
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > (nym_contracts_detailed) GET /v1/network/nym-contracts-detailed
 2023-12-12T14:29:55.800Z INFO  rocket::launch    > 📡 Fairings:
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > Validator Cache Stage (ignite)
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > Circulating Supply Cache Stage (ignite)
 2023-12-12T14:29:55.800Z INFO  rocket::launch::_ > Shield (liftoff, response, singleton)
 2023-12-12T14:29:55.801Z INFO  rocket::launch::_ > CORS (ignite, request, response)
 2023-12-12T14:29:55.801Z INFO  rocket::launch::_ > Node Status Cache (ignite)
 2023-12-12T14:29:55.801Z INFO  rocket::shield::shield > 🛡️ Shield:
 2023-12-12T14:29:55.801Z INFO  rocket::shield::shield::_ > X-Content-Type-Options: nosniff
 2023-12-12T14:29:55.801Z INFO  rocket::shield::shield::_ > X-Frame-Options: SAMEORIGIN
 2023-12-12T14:29:55.801Z INFO  rocket::shield::shield::_ > Permissions-Policy: interest-cohort=()
 2023-12-12T14:29:55.801Z WARN  rocket::launch            > 🚀 Rocket has launched from http://127.0.0.1:8000
 2023-12-12T14:29:56.375Z INFO  nym_api::nym_contract_cache::cache::refresher > Updating validator cache. There are 888 mixnodes and 105 gateways
 2023-12-12T14:29:56.375Z INFO  nym_api::node_status_api::cache::refresher    > Updating node status cache
 2023-12-12T14:29:57.359Z INFO  nym_api::circulating_supply_api::cache        > Updating circulating supply cache
 2023-12-12T14:29:57.359Z INFO  nym_api::circulating_supply_api::cache        > the mixmining reserve is now 220198535489690unym
 2023-12-12T14:29:57.359Z INFO  nym_api::circulating_supply_api::cache        > the number of tokens still vesting is now 145054386857730unym
 2023-12-12T14:29:57.359Z INFO  nym_api::circulating_supply_api::cache        > the circulating supply is now 634747077652580unym
 2023-12-12T14:30:00.803Z INFO  nym_api::support::caching::refresher          > node-self-described-data-refresher: refreshing cache state
 2023-12-12T14:31:56.290Z INFO  nym_api::nym_contract_cache::cache::refresher > Updating validator cache. There are 888 mixnodes and 105 gateways
 2023-12-12T14:31:56.291Z INFO  nym_api::node_status_api::cache::refresher    > Updating node status cache
```
~~~

## Automation
You will most likely want to automate your validator restarting if your server reboots. Checkout the [maintenance page](./maintenance.md) for an example `service` file.

## Exposing web endpoint using HTTPS
It is recommended to expose the webserver over HTTPS by using a webserver like Nginx. An example configuration for configuring Nginx is listed [on the maintenance page](maintenance.md#nym-api-configuration)
