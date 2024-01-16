# NymVPN alpha CLI: Guide for GNU/Linux

```admonish warning
NymVPN is an experimental software and it's for [testing](testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the event), and follow the steps listed in the form [*NymVPN User research*](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2).
```

## Installation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

1. Open Github [releases page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) and download the binary for Debian based Linux
2. Open terminal in the same directory and check the the `sha256sum` by running:
```sh
sha256sum ./nym-vpn-cli_0.1.0_ubuntu-22.04_amd64.zip
```
2. Compare the output with the sha256 hash shared on the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2)
3. Extract files with `unzip` command or manually as you are used to
4. Make executable by running:
```sh
chmod u+x ./nym-vpn-cli
```
5. Create Sandbox environment config file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `sandbox.env` in the same directory as your NymVPN binaries by running:
```sh
curl -o sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env
```

## Run NymVPN

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

Make sure your terminal is open in the same directory as your `nym-vpn-cli` binary.

1. Go to [nymvpn.com/en/alpha](https://nymvpn.com/en/alpha) to get the entire command with all the needed arguments' values and your wireguard private key for testing purposes 
2. Run it as root with `sudo` - the command will look like this with specified arguments:
```sh
sudo ./nym-vpn-cli -c ./sandbox.env --entry-gateway-id <ENTRY_GATEWAY_ID> --exit-router-address <EXIT_ROUTER_ADDRESS> --enable-wireguard --private-key <PRIVATE_KEY> --wg-ip <WIREGUARD_IP>
```
3. To chose different Gateways, visit [nymvpn.com/en/alpha/api/gateways](https://nymvpn.com/en/alpha/api/gateways) and pick one
4. See all possibilities in [command explanation](#cli-commands-and-options) section below

In case of errors, see [troubleshooting section](troubleshooting.md).

### CLI Commands and Options

The basic syntax of `nym-vpn-cli` is:
```sh
sudo ./nym-vpn-cli -c ./sandbox.env --entry-gateway-id <ENTRY_GATEWAY_ID> --exit-router-address <EXIT_ROUTER_ADDRESS> --enable-wireguard --private-key <PRIVATE_KEY> --wg-ip <WG_IP>
```
* To chose different Gateways, visit [nymvpn.com/en/alpha/api/gateways](https://nymvpn.com/en/alpha/api/gateways)
* To see all possibilities run with `--help` flag:
```sh
./nym-vpn-cli --help
```
~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ./binaries/nym-vpn-cli --help -->
```
~~~

**Fundamental commands and arguments**  

Here is a list of the options and their descriptions. Some are essential, some are more technical and not needed to adjusted by users:

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
