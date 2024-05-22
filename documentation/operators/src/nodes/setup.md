# Nym Node Setup & Run

This documentation page provides a guide on how to set up and run a [NYM NODE](nym-node.md), along with explanations of available flags, commands, and examples.

## Current version

```
<!-- cmdrun ../../../../target/release/nym-node --version -->
```

```admonish info
**Migrating an existing node to a new `nym-node` is simple. The steps are documented [below](#migrate).**
```

```admonish note
If you are a `nym-mixnode` or `nym-gateway` operator and you are not familiar with the binary changes called *Project Smoosh*, you can read the archived [Smoosh FAQ](../archive/faq/smoosh-faq.md) page.
```

## Summary

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

To run a new node, you can simply execute the `nym-node` command without any flags. By default, the node will set necessary configurations. If you later decide to change a setting, you can use the `-w` flag.

The most crucial aspect of running the node is specifying the `--mode`, which can be one of three: `mixnode`, `entry-gateway`, and `exit-gateway`.

Currently `nym-node` binary enables to run only one `--mode` at a time. In the future the operators will be able to specify multiple modes within one `nym-node`. Our goal is to have as many nodes each running all the available modes enabled and let the Nym API to position the node acoording the network needs in the beginning of each epoch.

Every `exit-gateway` mode is basically an `entry-gateway` with NR (Network Requester) and IPR (IP Packet Router) enabled. This means that every `exit-gateway` is automatically seen as an `entry-gateway` but not the opposite.

Gateway operators can check out the performance, connectivity and much more in our new tool [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net/).

To determine which mode your node is running, you can check the `:8080/api/v1/roles` endpoint. For example:
```
# for http
http://<IP_ADDRESS>:8080/api/v1/roles

# for https reversed proxy
https://<DOMAIN>/api/v1/roles
```

Everything necessary will exist on your node by default. For instance, if you're running a mixnode, you'll find that a NR (Network Requester) and IPR (IP Packet Router) address exist, but they will be ignored in `mixnode` mode.

For more information about available endpoints and their status, you can refer to:
```
# for http
http://<IP>:8080/api/v1/swagger/#/

# for https reversed proxy
https://<DOMAIN>/api/v1/swagger/#/
```

## Usage

### Help Command

There are a few changes from the individual binaries used in the past. For example by default `run` command does `init` function as well, local node `--id` will be set by default unless specified otherwise etcetera.

Run `./nym-node --help` to see all available commands:

~~~admonish example collapsible=true title="`./nym-node --help` output:"
```
<!-- cmdrun ../../../../target/release/nym-node --help -->
```
~~~

<!--
IN CASE CMD-RUN DOESN'T WORK HAR PASTE THIS:

Usage: nym-node [OPTIONS] <COMMAND>

Commands:
  build-info           Show build information of this binary
  bonding-information  Show bonding information of this node depending on its currently selected mode
  node-details         Show details of this node
  migrate              Attempt to migrate an existing mixnode or gateway into a nym-node
  run                  Start this nym-node
  sign                 Use identity key of this node to sign provided message
  help                 Print this message or the help of the given subcommand(s)

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file that configures the nym-node and overrides any preconfigured values [env: NYMNODE_CONFIG_ENV_FILE_ARG=]
      --no-banner
          Flag used for disabling the printed banner in tty [env: NYMNODE_NO_BANNER=]
  -h, --help
          Print help
  -V, --version
          Print version
-->

To list all available flags for each command, run `./nym-node <COMMAND> --help` for example `./nym-node run --help`:

~~~admonish example collapsible=true title="`./nym-node run --help` output:"
```
<!-- cmdrun ../../../../target/release/nym-node run --help  -->
```
~~~

<!-- IN CASE CMD-RUN DOESN'T WORK HAR PASTE THIS:

Start this nym-node

Usage: nym-node run [OPTIONS]

Options:
      --id <ID>
          Id of the nym-node to use [env: NYMNODE_ID=] [default: default-nym-node]
      --config-file <CONFIG_FILE>
          Path to a configuration file of this node [env: NYMNODE_CONFIG=]
      --deny-init
          Forbid a new node from being initialised if configuration file for the provided specification doesn't already exist [env: NYMNODE_DENY_INIT=]
      --init-only
          If this is a brand new nym-node, specify whether it should only be initialised without actually running the subprocesses [env: NYMNODE_INIT_ONLY=]
      --mode <MODE>
          Specifies the current mode of this nym-node [env: NYMNODE_MODE=] [possible values: mixnode, entry-gateway, exit-gateway]
  -w, --write-changes
          If this node has been initialised before, specify whether to write any new changes to the config file [env: NYMNODE_WRITE_CONFIG_CHANGES=]
      --bonding-information-output <BONDING_INFORMATION_OUTPUT>
          Specify output file for bonding information of this nym-node, i.e. its encoded keys. NOTE: the required bonding information is still a subject to change and this argument should be treated only as a preview of future features [env: NYMNODE_BONDING_INFORMATION_OUTPUT=]
  -o, --output <OUTPUT>
          Specify the output format of the bonding information (`text` or `json`) [env: NYMNODE_OUTPUT=] [default: text] [possible values: text, json]
      --public-ips <PUBLIC_IPS>
          Comma separated list of public ip addresses that will be announced to the nym-api and subsequently to the clients. In nearly all circumstances, it's going to be identical to the address you're going to use for bonding [env: NYMNODE_PUBLIC_IPS=]
      --hostname <HOSTNAME>
          Optional hostname associated with this gateway that will be announced to the nym-api and subsequently to the clients [env: NYMNODE_HOSTNAME=]
      --location <LOCATION>
          Optional **physical** location of this node's server. Either full country name (e.g. 'Poland'), two-letter alpha2 (e.g. 'PL'), three-letter alpha3 (e.g. 'POL') or three-digit numeric-3 (e.g. '616') can be provided [env: NYMNODE_LOCATION=]
      --http-bind-address <HTTP_BIND_ADDRESS>
          Socket address this node will use for binding its http API. default: `0.0.0.0:8080` [env: NYMNODE_HTTP_BIND_ADDRESS=]
      --landing-page-assets-path <LANDING_PAGE_ASSETS_PATH>
          Path to assets directory of custom landing page of this node [env: NYMNODE_HTTP_LANDING_ASSETS=]
      --http-access-token <HTTP_ACCESS_TOKEN>
          An optional bearer token for accessing certain http endpoints. Currently only used for prometheus metrics [env: NYMNODE_HTTP_ACCESS_TOKEN=]
      --expose-system-info <EXPOSE_SYSTEM_INFO>
          Specify whether basic system information should be exposed. default: true [env: NYMNODE_HTTP_EXPOSE_SYSTEM_INFO=] [possible values: true, false]
      --expose-system-hardware <EXPOSE_SYSTEM_HARDWARE>
          Specify whether basic system hardware information should be exposed. default: true [env: NYMNODE_HTTP_EXPOSE_SYSTEM_HARDWARE=] [possible values: true, false]
      --expose-crypto-hardware <EXPOSE_CRYPTO_HARDWARE>
          Specify whether detailed system crypto hardware information should be exposed. default: true [env: NYMNODE_HTTP_EXPOSE_CRYPTO_HARDWARE=] [possible values: true, false]
      --mixnet-bind-address <MIXNET_BIND_ADDRESS>
          Address this node will bind to for listening for mixnet packets default: `0.0.0.0:1789` [env: NYMNODE_MIXNET_BIND_ADDRESS=]
      --nym-api-urls <NYM_API_URLS>
          Addresses to nym APIs from which the node gets the view of the network [env: NYMNODE_NYM_APIS=]
      --nyxd-urls <NYXD_URLS>
          Addresses to nyxd chain endpoint which the node will use for chain interactions [env: NYMNODE_NYXD=]
      --wireguard-enabled <WIREGUARD_ENABLED>
          Specifies whether the wireguard service is enabled on this node [env: NYMNODE_WG_ENABLED=] [possible values: true, false]
      --wireguard-bind-address <WIREGUARD_BIND_ADDRESS>
          Socket address this node will use for binding its wireguard interface. default: `0.0.0.0:51822` [env: NYMNODE_WG_BIND_ADDRESS=]
      --wireguard-private-network-ip <WIREGUARD_PRIVATE_NETWORK_IP>
          Ip address of the private wireguard network. default: `10.1.0.0` [env: NYMNODE_WG_IP_NETWORK=]
      --wireguard-announced-port <WIREGUARD_ANNOUNCED_PORT>
          Port announced to external clients wishing to connect to the wireguard interface. Useful in the instances where the node is behind a proxy [env: NYMNODE_WG_ANNOUNCED_PORT=]
      --wireguard-private-network-prefix <WIREGUARD_PRIVATE_NETWORK_PREFIX>
          The prefix denoting the maximum number of the clients that can be connected via Wireguard. The maximum value for IPv4 is 32 and for IPv6 is 128 [env: NYMNODE_WG_PRIVATE_NETWORK_PREFIX=]
      --verloc-bind-address <VERLOC_BIND_ADDRESS>
          Socket address this node will use for binding its verloc API. default: `0.0.0.0:1790` [env: NYMNODE_VERLOC_BIND_ADDRESS=]
      --entry-bind-address <ENTRY_BIND_ADDRESS>
          Socket address this node will use for binding its client websocket API. default: `0.0.0.0:9000` [env: NYMNODE_ENTRY_BIND_ADDRESS=]
      --announce-ws-port <ANNOUNCE_WS_PORT>
          Custom announced port for listening for websocket client traffic. If unspecified, the value from the `bind_address` will be used instead [env: NYMNODE_ENTRY_ANNOUNCE_WS_PORT=]
      --announce-wss-port <ANNOUNCE_WSS_PORT>
          If applicable, announced port for listening for secure websocket client traffic [env: NYMNODE_ENTRY_ANNOUNCE_WSS_PORT=]
      --enforce-zk-nyms <ENFORCE_ZK_NYMS>
          Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet or if it also accepts non-paying clients [env: NYMNODE_ENFORCE_ZK_NYMS=] [possible values: true, false]
      --mnemonic <MNEMONIC>
          Custom cosmos wallet mnemonic used for zk-nym redemption. If no value is provided, a fresh mnemonic is going to be generated [env: NYMNODE_MNEMONIC=]
      --upstream-exit-policy-url <UPSTREAM_EXIT_POLICY_URL>
          Specifies the url for an upstream source of the exit policy used by this node [env: NYMNODE_UPSTREAM_EXIT_POLICY=]
      --open-proxy <OPEN_PROXY>
          Specifies whether this exit node should run in 'open-proxy' mode and thus would attempt to resolve **ANY** request it receives [env: NYMNODE_OPEN_PROXY=] [possible values: true, false]
  -h, --help
          Print help
-->

The Wireguard flags currently have limited functionality, **do not enable Wireguard** just yet. 

#### Flags Summary

Some of the most useful flags and their explanation:

- `--id <YOUR_ID>`: Local identifier of your node. This `<ID>` determines your config path located at `~/.nym/nym-nodes/<ID>/config/config.toml`, default value is `default-nym-node`
- `--config-file <PATH>`: Used for the migrate command to indicate the location of the existing node config file. Default path is `~/.nym/nym-nodes/default-nym-node/config/config.toml`
- `--deny-init`: Use this flag to prevent a new node from being initialized. It's recommended to use this after the first run to avoid accidental spinning up of a second node.
- `--init-only`: Use this flag if you want to set up a node without starting it.
- `--mode`: Determines the mode of the node and is always required.
- `--write-changes`: Used to change values within the `config.toml` file after the node has been run.
- `--mnemonic`: This is for when gateways are coconut-credentials-enforced, and this mnemonic is used as the `double_spend` prevention. This account needs credit in order for it to work.
- `--expose-system-info <true/false>`: Sets your system info visibility on the network.
- `--expose-system-hardware <true/false>`: Sets your system hardware info visibility on the network.
- `--expose-crypto-hardware <true/false>`: Sets your crypto hardware info visibility on the network.


## Commands & Examples

**`nym-node` introduces a default human readible ID (local only) `default-nym-node`, which is used if there is not an explicit custom `--id <ID>` specified. All configuration is stored in `~/.nym/nym-nodes/default-nym-node/config/config.toml` or `~/.nym/nym-nodes/<ID>/config/config.toml` respectively.**

### Initialise & Run

When we use `run` command the node will do `init` as well, unless we specify with a flag `--deny-init`. Below are some examples of initialising and running `nym-node` with different modes (`--mode`) like `mixnode`, `entry-gateway`, `exit-gateway`.

Please keep in mind that currently you can run only one functionality (`--mode`) per a `nym-node` instance. We are yet to finalise implement the multi-functionality solution under one node bonded to one Nyx account. Every `exit-gateway` can function as `entry-gateway` by default, not vice versa.

```admonish success title=""
**We recommend operators to setup an [automation](configuration.md#systemd) flow for their nodes!**

In such case, you can `run` a node to initalise it or try if everything works, but then stop the proces and paste your entire `run` command syntax (below) to the `ExecStart` line of your `/etc/systemd/system/nym-node.service` and start the node as a [service](configuration.md#following-steps-for-nym-nodes-running-as-systemd-service).
```

```admonish note
To prevent over-flooding of our documentation we cannot provide with every single command syntax as there is a large combination of possibilities. Please use a common sense and the explanation in `--help` option.
```

#### Mode: `exit-gateway`

As part of the transition, `allowed.list` on Exit Gateway embedded Network Requester was depreciated.

**Initialise and run** in one command:
```sh
# simple default
./nym-node  run  --mode exit-gateway

# with other options
./nym-node run --id <ID> --mode exit-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --location <COUNTRY_FULL_NAME> --wireguard-enabled false

# <YOUR_DOMAIN> is in format without 'https://' prefix
# <COUNTRY_FULL_NAME> is format like 'Jamaica',  or two-letter alpha2 (e.g. 'JM'), three-letter alpha3 (e.g. 'JAM') or three-digit numeric-3 (e.g. '388') can be provided.
# keep wireguard disabled
```

**Initialise only** without running the node with `--init-only` command :

```sh
# simple default
./nym-node run --init-only --mode exit-gateway

# with a custom `--id` and other options
./nym-node run --id <ID> --init-only --mode exit-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 true --location <COUNTRY_FULL_NAME> --wireguard-enabled false

# <YOUR_DOMAIN> is in format without 'https://' prefix
# <COUNTRY_FULL_NAME> is format like 'Jamaica',  or two-letter alpha2 (e.g. 'JM'), three-letter alpha3 (e.g. 'JAM') or three-digit numeric-3 (e.g. '388') can be provided.
# keep wireguard disabled
```

Run the node with custom `--id` without initialising, using `--deny-init` command
```sh
./nym-node run --id <ID> --deny-init --mode exit-gateway
```

#### Mode: `entry-gateway`

**Initialise and run:**
```sh
./nym-node run --mode entry-gateway
```

Initialise only with a custom `--id` and `--init-only` command:
```sh
./nym-node run --id <ID> --init-only --mode entry-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789
```

Run the node with custom `--id` without initialising:
```sh
./nym-node run --id <ID> --deny-init --mode entry-gateway
```

#### Mode: `mixnode`

**Initialise and run:**
```sh
./nym-node run --mode mixnode
```

Initialise only with a custom `--id` and `--init-only` command:
```sh
./nym-node run --id <ID> --init-only --mode mixnode --verloc-bind-address 0.0.0.0:1790 --public-ips "$(curl -4 https://ifconfig.me)"
```

Run the node with custom `--id` without initialising:
```sh
./nym-node run --id <ID> --deny-init --mode mixnode
```

Run the node with custom `--id` without initialising:
```sh
./nym-node run --id <ID> --deny-init --mode entry-gateway
```

### Migrate

```admonish caution
Migration is a must for all deprecated nodes (`nym-mixnode`, `nym-gateway`). For backward compatibility we created an [archive section](../archive/nodes/setup-guides.md) with all the guides for individual binaries. However, the binaries from version 1.1.35 (`nym-gateway`) and 1.1.37 (`nym-mixnode`) onwards will no longer have `init` command.
```

Operators who are about to migrate their nodes need to configure their [VPS](vps-setup.md) and setup `nym-node` which can be downloaded as a [pre-built binary](../binaries/pre-built-binaries.md) or compiled from [source](../binaries/building-nym.md).

To migrate a `nym-mixnode` or a `nym-gateway` to `nym-node` is fairly simple, use the `migrate` command with `--config-file` flag pointing to the original `config.toml` file, with a conditional argument defining which type of node this configuration belongs to. Examples are below.

Make sure to use `--deny-init` flag to prevent initialisation of a new node.

#### Mode: `mixnode`
```sh
# move relevant infor from config.toml
./nym-node migrate --config-file ~/.nym/mixnodes/<MIXNODE_ID>/config/config.toml mixnode

# initialise with the new nym-node config
./nym-node run --mode mixnode --id <NYM-NODE_ID> --deny-init
```

#### Mode: `entry-gateway` and `exit-gateway`
```sh
# move relevant infor from config.toml
./nym-node migrate --config-file ~/.nym/gateways/<GATEWAY_ID>/config/config.toml gateway

# initialise with the new nym-node config
./nym-node run --mode exit-gateway --id <NYM-NODE_ID> --deny-init # or change to entry-gateway
```

### Next steps

If there are any problems checkout the troubleshooting section or report an issue.

Follow up with [configuration](configuration.md) page for automation, reversed proxy setup and other tweaks, then head straight to [bonding](bonding.md) page to finalise your setup.
