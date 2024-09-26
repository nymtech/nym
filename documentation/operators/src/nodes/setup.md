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

Currently the `nym-node` binary can only be run in a single `--mode` at any one time. In the future however, operators will be able to specify multiple modes that a single `nym-node` binary can run. Our goal is to have as many nodes as possible enabling multiple modes, and allow the Nym API to position the node according the network's needs in the beginning of each epoch.

Every `exit-gateway` mode is basically an `entry-gateway` with NR (Network Requester) and IPR (IP Packet Router) enabled. This means that every `exit-gateway` is automatically seen as an `entry-gateway` but not the opposite.

Gateway operators can check out the node performance, connectivity and much more in our new tool [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net/).

To determine which mode your node is running, you can check the `:8080/api/v1/roles` endpoint. For example:
```sh
# sustitude <NODE_IP_ADDRESS> or <NODE_DOMAIN> with a real one
# for http
http://<NODE_IP_ADDRESS>:8080/api/v1/roles
# or
http://<NODE_IP_ADDRESS>/api/v1/roles

# for reversed proxy/WSS
https://<NODE_DOMAIN>/api/v1/roles
```

Everything necessary will exist on your node by default. For instance, if you're running a mixnode, you'll find that a NR (Network Requester) and IPR (IP Packet Router) address exist, but they will be ignored in `mixnode` mode.

For more information about available endpoints and their status, you can refer to:
```sh
# sustitude <NODE_IP_ADDRESS> or <NODE_DOMAIN> with a real one
# for http
http://<NODE_IP_ADDRESS>:8080/api/v1/swagger/#/
# or
http://<NODE_IP_ADDRESS>/api/v1/swagger/#/

# for reversed proxy/WSS
https://<NODE_DOMAIN>/api/v1/swagger/#/
```

## Usage

### Help Command

There are a few changes from the individual binaries used in the past. For example by default `run` command does `init` function as well, local node `--id` will be set by default unless specified otherwise etcetera.

```admonish info
You can always use `--help` flag to see the commands or arguments associated with a given command.
```

Run `./nym-node --help` to see all available commands:

~~~admonish example collapsible=true title="`./nym-node --help` output:"
```
<!-- cmdrun ../../../../target/release/nym-node --help -->
```
~~~

To list all available flags for each command, run `./nym-node <COMMAND> --help` for example `./nym-node run --help`:

~~~admonish example collapsible=true title="`./nym-node run --help` output:"
```
<!-- cmdrun ../../../../target/release/nym-node run --help  -->
```
~~~

```admonish warning
The Wireguard flags currently have limited functionality. From version `1.1.6` ([`v2024.9-topdeck`](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.9-topdeck)) wireguard is available and recommended to be switched on for nodes running as Gateways. Keep in mind that this option needs a bit of a special [configuration](configuration.md#wireguard-setup).
```

### Terms & Conditions

```admonish info
From `nym-node` version `1.1.3` onward is required to accept [**Operators Terms & Conditions**]({{toc_page}}) in order to be part of the active set. Make sure to read them before you add the flag.
```

There has been a long ongoing discussion whether and how to apply Terms and Conditions for Nym network operators, with an aim to stay aligned with the philosophy of Free Software and provide legal defense for both node operators and Nym developers. To understand better the reasoning behind this decision, you can listen to the first [Nym Operator Town Hall](https://www.youtube.com/live/7hwb8bAZIuc?si=3mQ2ed7AyUA1SsCp&t=915) introducing the T&Cs or to [Operator AMA with CEO Harry Halpin](https://www.youtube.com/watch?v=yIN-zYQw0I0) from June 4th, 2024, explaining pros and cons of T&Cs implementation.

Accepting T&Cs is done via a flag `--accept-operator-terms-and-conditions` added explicitly to `nym-node run` command every time. If you use [systemd](configuration.md#systemd) automation, add the flag to your service file's `ExecStart` line.

To check whether any node has T&Cs accepted or not can be done by querying Swagger API endpoint `/auxiliary_details` via one of these ports (depending on node setup):
```sh
# sustitude <NODE_IP_ADDRESS> or <NODE_DOMAIN> with a real one
http://<NODE_IP_ADDRESS>:8080/api/v1/auxiliary_details
https://<NODE_DOMAIN>/api/v1/auxiliary_details
http://<NODE_IP_ADDRESS>/api/v1/auxiliary_details
```

~~~admonish example collapsible=true title="Example of `/auxiliary_details` query"
```sh
# substitude <NODE_IP_ADDRESS> with a real one
curl -X 'GET' \
  'http://<NODE_IP_ADDRESS>:8080/api/v1/auxiliary-details' \
  -H 'accept: application/json'

{
  "location": "Kurdistan",
  "accepted_operator_terms_and_conditions": true
}
```
~~~

### Commands & Examples

**`nym-node` introduces a default human readible ID (local only) `default-nym-node`, which is used if there is not an explicit custom `--id <ID>` specified. All configuration is stored in `~/.nym/nym-nodes/default-nym-node/config/config.toml` or `~/.nym/nym-nodes/<ID>/config/config.toml` respectively.**

```admonish info
All commands with more options listed below include `--accept-operator-terms-and-conditions` flag, read [Terms & Conditions](#terms--conditions) chapter above before executing these commands.
```

### Initialise & Run

When we use `run` command the node will do `init` as well, unless we specify with a flag `--deny-init`. Below are some examples of initialising and running `nym-node` with different modes (`--mode`) like `mixnode`, `entry-gateway`, `exit-gateway`.

Please keep in mind that currently you can run only one functionality (`--mode`) per a `nym-node` instance. We are yet to finalise implement the multi-functionality solution under one node bonded to one Nyx account. Every `exit-gateway` can function as `entry-gateway` by default, not vice versa.

There is a simple default command to initialise and run your node: `./nym-node  run  --mode <MODE>`, however there quite a few parameters to be configured. When `nym-node` gets to be `run`, these parameters are read by the binary from the configuration file located at `.nym/nym-nodes/<ID>/config/config.toml`.

If an operator specifies any paramteres with optional flags alongside `run` command, these parameters passed in the option will take place over the ones in `config.toml` but they will not overwrite them by default. To overwrite them with the values passed with `run` command, a flag `-w` (`--write-changes`)  must be added.

Alternatively operators can just open a text editor and change these values manually. After saving the file,don't forget to restart the node or reload and restart the service. If all values are setup correctly in `config.toml`, then operator can use as simple command as `nym-node run --mode <MODE> --accept-operators-terms-and-conditions`, or alternatively paste this command with a correct path to your binary to your `ExecStart` line into a [systemd `nym-node.service`](configuration.md#systemd) config file.

```admonish success title=""
**We recommend operators to setup an [automation](configuration.md#systemd) flow for their nodes, using systemd!**

In such case, you can `run` a node to initalise it or try if everything works, but then stop the proces and paste your entire `run` command syntax (below) to the `ExecStart` line of your `/etc/systemd/system/nym-node.service` and start the node as a [service](configuration.md#following-steps-for-nym-nodes-running-as-systemd-service).
```

#### Essential Parameters & Variables

Running a `nym-node` in a `mixnode` mode requires less configuration than a full `exit-gateway` setup, we recommend operators to still follow through with all documented [configuration](configuration.md). Before you scroll down to syntax examples for the mode of your choice: [`mixnode`](#mode-miznode), [`exit-gateway`](#mode-exit-gateway), or [`entry-gateway](#mode-entry-gateway), please familiarise yourself with the essential paramters and variables convention we use in the guide.

Substitute any variables in `<>` brackets your own value, without `<>` brackets. Here is a list of important options and variables.

| Flag (Option)                            | Variable                                                                                                   | Description                                                                                                                                                                | Syntax example                                  |
| :--                                      | :---                                                                                                       | :---                                                                                                                                                                       | :---                                            |
| `--mode`                                 | `<MODE>`                                                                                                   | A functionality of your `nym-node` in the mixnet - mandatory! Chose from `entry-gateway`, `mixnode` or `exit-gateway`                                                      | `--mode exit-gateway`                           |
| `--id`                                   | `<ID>`                                                                                                     | A local only `nym-node` identifier, specified by flag `--id`. Not mandatory as it defaults to `default-nym-node` if not specified.                                         | `--id alice_super_node`                         |
| `-w` or `--write-changes`                | *none*                                                                                                     | Specify whether to write new changes - the values of other flags in the given command - to the config file                                                                 | `--write-changes`                               |
| `--accept-operator-terms-and-conditions` | *none*                                                                                                     | A flag added explicitly to `nym-node run` command every time, showing that the operator agreed with [T&Cs](#terms--conditions)                                             | `--accept-operator-terms-and-conditions`        |
| `--public-ips`                           | `<PUBLIC_IPS>`                                                                                             | IPv4 of the `nym-node` server - mandatory! Use this address as a `host` value for bonding.Use this address as a `host` value for bonding.                                  | `--public-ips "$(curl -4 https://ifconfig.me)"` |
| `--mixnet-bind-address`                  | `<MIXNET_BIND_ADDRESS>`                                                                                    | Address to bind to for listening for mixnet packets - mandatory! Must be on port `1789`!                                                                                   | `--mixnet-bind-address 0.0.0.0:1789`            |
| `--http-bind-address`                    | `<HTTP_BIND_ADDRESS>`                                                                                      | Socket address this node will use for binding its http API - mandatory! Must be on port `8080`!                                                                            | `--http-bind-address 0.0.0.0:8080`              |
| `--hostname`                             | `<HOSTNAME>`                                                                                               | Your registered DNS domain, asigned to the VPS with `nym-node`. Use without prefix like `http://` or `https://`                                                            | `exit-gateway1.squad.nsl`                       |
| `--location`                             | `<LOCATION>`                                                                                               | Loacation of your node. Formats like 'Jamaica',  or two-letter alpha2 (e.g. 'JM'), three-letter alpha3 (e.g. 'JAM') or three-digit numeric-3 (e.g. '388') can be provided. | `--location JAM`                                |
| `--wireguard-private-ip`                 | `<WIREGUARD_PRIVATE_IP>`                                                                                   | Private IP address of the wireguard gateway. This mandatory field is set to a correct default: `10.1.0.1`, operators upgrading from older versions must overwrite it.      | `--wireguard-private-ip 10.1.0.1`               |
| `--wireguard-enabled`                    | `<WIREGUARD_ENABLED>`                                                                                      | Specifies whether the wireguard service is enabled, possible values: `true` or `false` - `true` is recommended                                                             | `--wireguard-enabled true`                      |
| `--expose-system-info`                   | `<EXPOSE_SYSTEM_INFO>`                                                                                     | Specify whether basic system information should be exposed. default: `true`, possible values: `true` or `false`                                                            | `--expose-system-info false`                    |
| `--expose-system-hardware`               | `<EXPOSE_SYSTEM_HARDWARE>`                                                                                 | Specify whether basic system hardware information should be exposed. default: `true`, possible values: `true` or `false`                                                   | `--expose-system-hardware false`                |
| *not a flag*                             | `<PATH_TO>`                                                                                                | Specify a full path to the given file, directory or binary behind this variable                                                                                            | `/root/src/nym/target/release/`                 |

```admonish note
To prevent over-flooding of our documentation we cannot provide with every single command syntax as there is a large combination of possibilities. Please use a common sense and the explanation in `--help` option.
```

#### Mode: `exit-gateway`

If you run a `nym-node` for the first time, you will need to specify a few parameters, please read the section [Essential Parameters & Variables](#essential-paramteters--varibles) before you start and make sure that your `nym-node` is up to date with the [latest version](https://github.com/nymtech/nym/releases/).

**Initialise and run** in one command:

To initialise and test run your node, use this command:
```sh
./nym-node run --id <ID> --mode exit-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<HOSTNAME>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --location <LOCATION> --accept-operator-terms-and-conditions --wireguard-enabled true
```
If you prefer to have a generic local identifier set to `default-nym-node`, skip `--id` option.

We highly recommend to setup [reverse proxy and WSS](proxy-configuration.md) for `nym-node`. If you haven't configured any of that, skip `--hostname` flag.

In any case `--public-ips` is a necessity for your node to bond to API and communicate with the internet.


**Initialise only** without running the node with `--init-only` command:

Adding `--init-only` option results in `nym-node` initialising a configuration file `config.toml` without running - a good option for an initial node setup. Remember that if you using this flag on a node which already has a config file, this will not over-write the values, unless used with a specified flag `--write-changes` (`-w`) - a good option for introducing changes to your `config.toml` file.

```sh
./nym-node run --id <ID> --init-only --mode exit-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<HOSTNAME>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --location <LOCATION> --wireguard-enabled true
```

In the example above we dropped `--accept-operator-terms-and-conditions` as the flag must be added to a running command explicitly and it is not stored in the config, `--init-only` will not run the node.

**Deny init**

`--deny-init` was introduced as an additional safety for migration from legacy binaries to `nym-node` to prevent operators initialise over existing nodes. For most of the operators, this flag is not needed.

In this example we run the node with custom `--id` without initialising, using `--deny-init` command:
```sh
./nym-node run --id <ID> --deny-init --mode exit-gateway --accept-operator-terms-and-conditions
```

#### Mode: `entry-gateway`

If you run a `nym-node` for the first time, you will need to specify a few parameters, please read the section [Essential Parameters & Variables](#essential-paramteters--varibles) before you start and make sure that your `nym-node` is up to date with the [latest version](https://github.com/nymtech/nym/releases/).

**Initialise and run:**

To initialise and test run with yur node with all needed options, use this command:
```sh
./nym-node run --id <ID> --mode entry-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<HOSTNAME>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --location <LOCATION> --accept-operator-terms-and-conditions --wireguard-enabled true
```
If you prefer to have a generic local identifier set to `default-nym-node`, skip `--id` option.

We highly recommend to setup [reverse proxy and WSS](proxy-configuration.md) for `nym-node`. If you haven't configured any of that, skip `--hostname` flag.

In any case `--public-ips` is a necessity for your node to bond to API and communicate with the internet.

**Initialise only** without running the node with `--init-only` command :

Adding `--init-only` option results in `nym-node` initialising a configuration file `config.toml` without running - a good option for an initial node setup. Remember that if you using this flag on a node which already has a config file, this will not over-write the values, unless used with a specified flag `--write-changes` (`-w`) - a good option for introducing changes to your `config.toml` file.

```sh
./nym-node run --id <ID> --init-only --mode entry-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<HOSTNAME>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --location <LOCATION> --wireguard-enabled true
```

In the example above we dropped `--accept-operator-terms-and-conditions` as the flag must be added to a running command explicitly and it is not stored in the config, `--init-only` will not run the node.

**Deny init**

`--deny-init` was introduced as an additional safety for migration from legacy binaries to `nym-node` to prevent operators initialise over existing nodes. For most of the operators, this flag is not needed.

In this example we run the node with custom `--id` without initialising, using `--deny-init` command:
```sh
./nym-node run --id <ID> --deny-init --mode entry-gateway --accept-operator-terms-and-conditions
```

#### Mode: `mixnode`

**Initialise and run:**
```sh
./nym-node run --mode mixnode --mixnet-bind-address 0.0.0.0:1789 --verloc-bind-address 0.0.0.0:1790 --http-bind-address 0.0.0.0:8080 --public-ips "$(curl -4 https://ifconfig.me)" --accept-operator-terms-and-conditions
```

**Init only**

Adding `--init-only` option results in `nym-node` initialising a configuration file `config.toml` without running - a good option for an initial node setup. Remember that if you using this flag on a node which already has a config file, this will not over-write the values, unless used with a specified flag `--write-changes` (`-w`) - a good option for introducing changes to your `config.toml` file.

Initialise only with a custom `--id` and `--init-only` command:
```sh
./nym-node run --mode mixnode --id <ID> --init-only --mixnet-bind-address 0.0.0.0:1789 --verloc-bind-address 0.0.0.0:1790 --http-bind-address 0.0.0.0:8080 --public-ips "$(curl -4 https://ifconfig.me)" --accept-operator-terms-and-conditions
```

If you prefer to have a generic local identifier set to `default-nym-node`, skip `--id` option.

**Deny init**

`--deny-init` was introduced as an additional safety for migration from legacy binaries to `nym-node` to prevent operators initialise over existing nodes. For most of the operators, this flag is not needed.

In this example we run the node with custom `--id` without initialising, using `--deny-init` command:
```sh
./nym-node run --mode mixnode --id <ID> --deny-init --accept-operator-terms-and-conditions
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
./nym-node run --mode mixnode --id <NYM-NODE_ID> --accept-operator-terms-and-conditions
```

#### Mode: `entry-gateway` and `exit-gateway`
```sh
# move relevant infor from config.toml
./nym-node migrate --config-file ~/.nym/gateways/<GATEWAY_ID>/config/config.toml gateway

# initialise with the new nym-node config - entry-gateway
./nym-node run --mode entry-gateway --id <NYM-NODE_ID> --accept-operator-terms-and-conditions

# or as exit-gateway
./nym-node run --id <NYM-NODE_ID> --mode exit-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --location <COUNTRY_FULL_NAME> --accept-operator-terms-and-conditions --wireguard-enabled false
```

### Next steps

If there are any problems checkout the troubleshooting section or report an issue.

Follow up with [configuration](configuration.md) page for automation, reversed proxy setup and other tweaks, then head straight to [bonding](bonding.md) page to finalise your setup.
