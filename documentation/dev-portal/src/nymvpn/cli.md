# NymVPN CLI Guide

```admonish tip title="web3summit testing"
**If you testing NymVPN CLI on Web3 Summit Berlin, visit our event page with all info tailored for the event: [nym-vpn-cli.sandbox.nymtech.net](https://nym-vpn-cli.sandbox.nymtech.net/).**
```

```admonish info
To download NymVPN desktop version, visit [nymvpn.com/en/download](https://nymvpn.com/en/download).

NymVPN is an experimental software and it's for testing purposes only. Anyone can submit a registration to the private alpha round on [nymvpn.com](https://nymvpn.com/en).
```

## Overview

The core binaries consist of:

- **`nym-vpn-cli`**: Basic commandline client for running the vpn. This runs in the foreground.

- **`nym-vpnd`**: Daemon implementation of the vpn client that can run in the background and interacted with using `nym-vpnc`.

- **`nym-vpnc`**: The commandline client used to interact with `nym-vpnd`.


## Installation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

1. Open Github [releases page]({{nym_vpn_releases}}) and download the CLI latest binary for your system (labelled as `nym-vpn-core`)

2. Verify sha hash of your downloaded binary with the one listed on the [releases page]({{nym_vpn_releases}}). You can use a simple `shasum` command and compare strings (ie with Python) or run in the same directory the following command, exchanging `<SHA_STRING>` with the one of your binary, like in the example:
```sh
echo "<SHA_STRING>" | shasum -a 256 -c

# choose a correct one according to your binary, this is just an example
# echo "0e4abb461e86b2c168577e0294112a3bacd3a24bf8565b49783bfebd9b530e23  nym-vpn-cli_<!-- cmdrun ../../../scripts/cmdrun/nym_vpn_cli_version.sh -->_ubuntu-22.04_amd64.tar.gz" | shasum -a 256 -c
```

3. Extract files:
```sh
tar -xvf <BINARY>.tar.gz
# for example
# tar -xvf nym-vpn-cli_<!-- cmdrun ../../../scripts/cmdrun/nym_vpn_cli_version.sh -->_ubuntu-22.04_x86_64.tar.gz
```

### Building From Source

NymVPN CLI can be built from source. This process is recommended for more advanced users as the installation may require different dependencies based on the operating system used.

Start by installing [Go](https://go.dev/doc/install) and [Rust](https://rustup.rs/) languages on your system and then follow these steps:

1. Clone NymVPN repository:
```sh
git clone https://github.com/nymtech/nym-vpn-client.git
```

2. Move to `nym-vpn-client` directory and compile `wireguard`:

```sh
cd nym-vpn-client

make build-wireguard
```

3. Compile NymVPN CLI
```sh
make build-nym-vpn-core
```

Now your NymVPN CLI is installed. Navigate to `nym-vpn-core/target/release` and use the commands the section below to run the client.

## Running

If you are running Debian/Ubuntu/PopOS or any other distributio supporting debian packages and systemd, see the [relevant section below](#debian-package-for-debianubuntupopos).

### Daemon

Start the daemon with

```sh
sudo -E ./nym-vpnd
```

Then run

```sh
./nym-vpnc status
./nym-vpnc connect
./nym-vpnc disconnect
```

### CLI

An alternative to the daemon is to run the `nym-vpn-cli` commandline client that runs in the foreground.
```sh
./nym-vpn-cli run
```

## Credentials

NymVPN uses [zkNym bandwidth credentials](https://nymtech.net/docs/bandwidth-credentials.html). Those can be imported as a file or base58 encoded string.


```sh
sudo -E ./nym-vpn-cli import-credential --credential-path </PATH/TO/freepass.nym>
sudo -E ./nym-vpn-cli import-credential --credential-data "<STRING>"
```

## Debian package for Debian/Ubuntu/PopOS

For linux platforms using deb packages and systemd, there are also debian packages.

```sh
sudo apt install ./nym-vpnd_<!-- cmdrun ../../../scripts/cmdrun/nym_vpn_cli_version.sh -->-1_amd64.deb ./nym-vpnc_<!-- cmdrun ../../../scripts/cmdrun/nym_vpn_cli_version.sh -->-1_amd64.deb

# In case of error please substitute the correct version
```

Installing the `nym-vpnd` deb package starts a `nym-vpnd.service`. Check that the daemon is running with
```sh
systemctl status nym-vpnd.service
```
and check its logs with
```sh
sudo journalctl -u nym-vpnd.service -f
```
To stop the background service
```sh
systemctl stop nym-vpnd.service
```
It will start again on startup, so disable with
```sh
systemctl disable nym-vpnd.service
```

Interact with it with `nym-vpnc`
```sh
nym-vpnc status
nym-vpnc connect
nym-vpnc disconnect
```

## Commands & Options

```admonish note
Nym Exit Gateway functionality was implemented just recently and not all the Gateways are upgraded and ready to handle the VPN connections. If you want to make sure you are connecting to a Gateway with an embedded Network Requester, IP Packet Router and applied Nym exit policy, visit [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net/) and search Gateways with all the functionalities enabled.
```

The basic syntax of `nym-vpn-cli` is:
```sh
# choose only one conditional --argument listed in {brackets}
sudo ./nym-vpn-cli { --exit-router-address <EXIT_ROUTER_ADDRESS>|--exit-gateway-id <EXIT_GATEWAY_ID>|--exit-gateway-country <EXIT_GATEWAY_COUNTRY> }
```

To see all the possibilities run with `--help` flag:
```sh
./nym-vpn-cli --help
```

~~~admonish example collapsible=true title="nym-vpn-cli --help"
```sh
Usage: nym-vpn-cli [OPTIONS] <COMMAND>

Commands:
  run                Run the client
  import-credential  Import credential
  help               Print this message or the help of the given subcommand(s)

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>  Path pointing to an env file describing the network
      --data-path <DATA_PATH>              Path to the data directory of the mixnet client
  -h, --help                               Print help
  -V, --version                            Print version
```
~~~

You can also run any command with `--help` flag to see a list of all options associated witht that command, the most important may be `run` command, like in this example.

~~~admonish example collapsible=true title="nym-vpn-cli run --help"
```sh
Run the client

Usage: nym-vpn-cli run [OPTIONS]

Options:
      --entry-gateway-id <ENTRY_GATEWAY_ID>
          Mixnet public ID of the entry gateway
      --entry-gateway-country <ENTRY_GATEWAY_COUNTRY>
          Auto-select entry gateway by country ISO
      --entry-gateway-low-latency
          Auto-select entry gateway by latency
      --exit-router-address <EXIT_ROUTER_ADDRESS>
          Mixnet recipient address
      --exit-gateway-id <EXIT_GATEWAY_ID>
          Mixnet public ID of the exit gateway
      --exit-gateway-country <EXIT_GATEWAY_COUNTRY>
          Auto-select exit gateway by country ISO
      --wireguard-mode
          Enable the wireguard mode
      --nym-ipv4 <NYM_IPV4>
          The IPv4 address of the nym TUN device that wraps IP packets in sphinx packets
      --nym-ipv6 <NYM_IPV6>
          The IPv6 address of the nym TUN device that wraps IP packets in sphinx packets
      --nym-mtu <NYM_MTU>
          The MTU of the nym TUN device that wraps IP packets in sphinx packets
      --dns <DNS>
          The DNS server to use
      --disable-routing
          Disable routing all traffic through the nym TUN device. When the flag is set, the nym TUN device will be created, but to route traffic through it you will need to do it manually, e.g. ping -Itun0
      --enable-two-hop
          Enable two-hop mixnet traffic. This means that traffic jumps directly from entry gateway to exit gateway
      --enable-poisson-rate
          Enable Poisson process rate limiting of outbound traffic
      --disable-background-cover-traffic
          Disable constant rate background loop cover traffic
      --enable-credentials-mode
          Enable credentials mode
      --min-mixnode-performance <MIN_MIXNODE_PERFORMANCE>
          Set the minimum performance level for mixnodes
  -h, --help
          Print help

```
~~~


## Testnet environment

If you want to run NymVPN CLI in Nym Sandbox environment, there are a few adjustments to be done:

1. Create Sandbox environment config file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `sandbox.env` in the same directory as your NymVPN binaries:
```sh
curl -o sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env
```

2. Check available Gateways at [Sandbox API](https://sandbox-nym-api1.nymtech.net/api/v1/gateways) or [Sandbox Swagger page](https://sandbox-nym-api1.nymtech.net/api/swagger/index.html)

3. Run with a flag `-c`
```sh
sudo ./nym-vpn-cli -c <PATH_TO>/sandbox.env <--exit-router-address <EXIT_ROUTER_ADDRESS>|--exit-gateway-id <EXIT_GATEWAY_ID>|--exit-gateway-country <EXIT_GATEWAY_COUNTRY>>
```
