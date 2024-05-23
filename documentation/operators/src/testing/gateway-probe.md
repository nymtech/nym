# Nym Gateway Probe

Nym Node operators running Gateway functionality are already familiar with the monitoring tool [Harbourmaster.nymtech.net](https://harbourmaster.nymtech.net). Under the hood of Nym Harbourmaster runs iterations of `nym-gateway-probe` doing various checks and displaying the results on the interface. Operators don't have to rely on the probe ran by Nym and wait for the data to refresh. With `nym-gateway-probe` everyone can check any Gateway's networking status from their own computer at any time. In one command the client queries data from:

- [`nym-api`](https://validator.nymtech.net/api/)
- [`explorer-api`](https://explorer.nymtech.net/api/)
- [`harbour-master`](https://harbourmaster.nymtech.net/)


## Preparation

We recommend to have install all [the prerequisites](../binaries/building-nym.md#prerequisites) needed to build `nym-node` from source including latest [Rust Toolchain](https://www.rust-lang.org/tools/install).

## Installation

`nym-gateway-probe` source code is in [`nym-vpn-client`](https://github.com/nymtech/nym-vpn-client) repository. The client needs to be build from source.

1. Clone the repository:

```sh
git clone https://github.com/nymtech/nym-vpn-client.git
```

2. Build `nym-gateway-probe`:

```sh
cd nym-vpn-client

cargo build --release -p nym-gateway-probe
```

## Running the client

```sh
./target/release/nym-gateway-probe --help
```
~~~admonish collapsible=true
```
Usage: nym-gateway-probe [OPTIONS]

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>  Path pointing to an env file describing the network
  -g, --gateway <GATEWAY>
  -n, --no-log
  -h, --help                               Print help
  -V, --version                            Print version

```
~~~

To run the client, simply add a flag `--gateway` with a targeted gateway identity key. 

```sh
./target/release/nym-gateway-probe --gateway <GATEWAY_IDENTITY_KEY>
```

For any `nym-node --mode exit-gateway` the aim is to have this outcome:
```sh
{
  "gateway": "<GATEWAY_IDENTITY_KEY>",
  "outcome": {
    "as_entry": {
      "can_connect": true,
      "can_route": true
    },
    "as_exit": {
      "can_connect": true,
      "can_route_ip_v4": true,
      "can_route_ip_external_v4": true,
      "can_route_ip_v6": true,
      "can_route_ip_external_v6": true
    }
  }
}
```

If you don't provide a `--gateway` flag it will pick a random one to test.


