# NymVPN alpha CLI Guide

```admonish info
NymVPN is an experimental software and it's for testing purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the workshop) so we use their results to improve the client, and submit the form [*NymVPN User research*]({{nym_vpn_form_url}}) with the testing results.
```

## Installation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

1. Open Github [releases page](https://github.com/nymtech/nym-vpn-client) and download the CLI latest binary for your system

2. Verify sha hash of your downloaded binary with the one listed on the [releases page](https://github.com/nymtech/nym-vpn-client). You can use a simple `shasum` command and compare strings (ie with Python) or run in the same directory the following command, exchanging `<SHA_STRING>` with the one of your binary, like in the example:
```sh
echo "<SHA_STRING>" | shasum -a 256 -c

# choose a correct one according to your binary, this is just an example
# echo "0e4abb461e86b2c168577e0294112a3bacd3a24bf8565b49783bfebd9b530e23  nym-vpn-cli_<!-- cmdrun scripts/nym_vpn_cli_version.sh -->_ubuntu-22.04_amd64.tar.gz" | shasum -a 256 -c
```

3. Extract files:
```sh
tar -xvf <BINARY>.tar.gz
# for example
# tar -xvf nym-vpn-cli_<!-- cmdrun scripts/nym_vpn_cli_version.sh -->_ubuntu-22.04_x86_64.tar.gz
```

4. Make executable:
```sh
# make sure you are in the right sub-directory
chmod u+x ./nym-vpn-cli
```

## Run NymVPN

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

Make sure your terminal is open in the same directory as your `nym-vpn-cli` binary.

1. Go to [nymvpn.com/en/alpha](https://nymvpn.com/en/alpha) to get the entire command with all the needed arguments' values and your wireguard private key for testing purposes
2. Run it as root with `sudo` - the command will look like this with specified arguments:
```sh
sudo ./nym-vpn-cli -c ./sandbox.env --entry-gateway-id <ENTRY_GATEWAY_ID> --exit-router-address <EXIT_ROUTER_ADDRESS> --enable-wireguard --private-key <PRIVATE_KEY> --wg-ip <WIREGUARD_IP>
```
3. To choose different Gateways, visit [explorer.nymtech.net/network-components/gateways](https://explorer.nymtech.net/network-components/gateways) and copy-paste an identity key of your choice
4. See all possibilities in [command explanation](#cli-commands-and-options) section below

In case of errors, see [troubleshooting section](troubleshooting.md).

### CLI Commands and Options

The basic syntax of `nym-vpn-cli` is:
```sh
sudo ./nym-vpn-cli <--exit-router-address <EXIT_ROUTER_ADDRESS>|--exit-gateway-id <EXIT_GATEWAY_ID>|--exit-gateway-country <EXIT_GATEWAY_COUNTRY>>
```
* To choose different Gateways, visit [nymvpn.com/en/alpha/api/gateways](https://explorer.nymtech.net/network-components/gateways)
* To see all possibilities run with `--help` flag:
```sh
./nym-vpn-cli --help
```
~~~admonish example collapsible=true title="Console output"
```sh
Usage: nym-vpn-cli [OPTIONS] <--exit-router-address <EXIT_ROUTER_ADDRESS>|--exit-gateway-id <EXIT_GATEWAY_ID>|--exit-gateway-country <EXIT_GATEWAY_COUNTRY>>

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file describing the network
      --mixnet-client-path <MIXNET_CLIENT_PATH>
          Path to the data directory of a previously initialised mixnet client, where the keys reside
      --entry-gateway-id <ENTRY_GATEWAY_ID>
          Mixnet public ID of the entry gateway
      --entry-gateway-country <ENTRY_GATEWAY_COUNTRY>
          Auto-select entry gateway by country ISO
      --entry-gateway-low-latency
          Auto-select entry gateway by latency
      --exit-router-address <EXIT_ROUTER_ADDRESS>
          Mixnet recipient address
      --exit-gateway-id <EXIT_GATEWAY_ID>

      --exit-gateway-country <EXIT_GATEWAY_COUNTRY>
          Mixnet recipient address
      --enable-wireguard
          Enable the wireguard traffic between the client and the entry gateway
      --private-key <PRIVATE_KEY>
          Associated private key
      --wg-ip <WG_IP>
          The IP address of the wireguard interface used for the first hop to the entry gateway
      --nym-ipv4 <NYM_IPV4>
          The IPv4 address of the nym TUN device that wraps IP packets in sphinx packets
      --nym-ipv6 <NYM_IPV6>
          The IPv6 address of the nym TUN device that wraps IP packets in sphinx packets
      --nym-mtu <NYM_MTU>
          The MTU of the nym TUN device that wraps IP packets in sphinx packets
      --disable-routing
          Disable routing all traffic through the nym TUN device. When the flag is set, the nym TUN device will be created, but to route traffic through it you will need to do it manually, e.g. ping -Itun0
      --enable-two-hop
          Enable two-hop mixnet traffic. This means that traffic jumps directly from entry gateway to exit gateway
      --enable-poisson-rate
          Enable Poisson process rate limiting of outbound traffic
      --disable-background-cover-traffic
          Disable constant rate background loop cover traffic
  -h, --help
          Print help
  -V, --version
          Print version
```
~~~

Here is a list of the options and their descriptions. Some are essential, some are more technical and not needed to be adjusted by users.

**Fundamental commands and arguments**

- `-c` is a path to the [Sandbox config](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) file saved as `sandbox.env`
- `--entry-gateway-id`: paste one of the values labeled with a key `"identityKey"` (without `" "`) from [here](https://nymvpn.com/en/alpha/api/gateways)
- `--exit-router-address`: paste one of the values labeled with a key `"address"` (without `" "`) from here [here](https://nymvpn.com/en/alpha/api/gateways)
- `--enable-wireguard`: Enable the wireguard traffic between the client and the entry gateway. NymVPN uses Mullvad libraries for wrapping `wireguard-go` and to setup local routing rules to route all traffic to the TUN virtual network device
- `--wg-ip`: The address of the wireguard interface, you can get it [here](https://nymvpn.com/en/alpha)
- `--private-key`: get your private key for testing purposes [here](https://nymvpn.com/en/alpha)
- `--enable-two-hop` is a faster setup where the traffic is routed from the client to Entry Gateway and directly to Exit Gateway (default is 5-hops)

**Advanced options**

- `--enable-poisson`: Enables process rate limiting of outbound traffic (disabled by default). It means that NymVPN client will send packets at a steady stream to the Entry Gateway. By default it's on average one sphinx packet per 20ms, but there is some randomness (poisson distribution). When there are no real data to fill the sphinx packets with, cover packets are generated instead.
- `--ip` is the IP address of the TUN device. That is the IP address of the local private network that is set up between local client and the Exit Gateway.
- `--mtu`: The MTU of the TUN device. That is the max IP packet size of the local private network that is set up between local client and the Exit Gateway.
- `--disable-routing`: Disable routing all traffic through the VPN TUN device.

## Testnet environment

If you want to run NymVPN CLI in Nym Sandbox environment, there are a few adjustments to be done:

1. Create Sandbox environment config file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `sandbox.env` in the same directory as your NymVPN binaries by running:
```sh
curl -o sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env
```

1. Check available Gateways at [nymvpn.com/en/alpha/api/gateways](https://nymvpn.com/en/alpha/api/gateways)

2. Run with a flag `-c`
```sh
sudo ./nym-vpn-cli -c <PATH_TO>/sandbox.env <--exit-router-address <EXIT_ROUTER_ADDRESS>|--exit-gateway-id <EXIT_GATEWAY_ID>|--exit-gateway-country <EXIT_GATEWAY_COUNTRY>>
```
