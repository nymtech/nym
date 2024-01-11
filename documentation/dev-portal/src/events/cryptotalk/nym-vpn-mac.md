# NymVPN alpha: Guide for Mac OS

```admonish warning
NymVPN is an experimental software and it's for [testing](./nym-vpn-testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the event), and follow the steps listed in the form [*NymVPN User research*](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2).
```

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

### CLI Installation

You can use our automated script of follow the steps below to download, verify, install and run NymVPN client. 

#### Automated Script CLI Installation

Open a terminal and follow these steps:

* Download the script
```sh
curl -o nym-vpn-client-executor.sh -L https://gist.githubusercontent.com/tommyv1987/87267ded27e1eb7651aa9cc745ddf4af/raw/99cea8f4d80f2d002802ed1cbedba288bfca4488/execute-nym-vpn-cli-binary.sh
```
* Make the script executable
```sh
chmod u+x nym-vpn-client-executor.sh
```
* Run the script
```sh
./nym-vpn-client-executor.sh
```
* When prompted to verify `sha256sum` paste one from the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) including the binary name (all as one input with a space in between), for example:
```sh
96623ccc69bc4cc0e4e3e18528b6dae6be69f645d0a592d926a3158ce2d0c269  nym-vpn-cli_0.1.0_macos_x86_64.zip
```

The script will automatically start the client. Follow the instructions:

* The script will print a JSON view of existing Gateways and prompt you to:
    - ***(Make sure to use two different Gateways for entry and exit!)***
    - `enter a gateway ID:` paste one of the values labeled with a key `"identityKey"` printed above (without `" "`)
    - `enter an exit address:` paste one of the values labeled with a key `"address"` printed above (without `" "`)
    - `do you want five hop or two hop?`: type `five` or `two` 

#### Manual CLI Installation

* Visit the [releases page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) to download the binary for Debian based Linux
* Open terminal in the same directory and check the the `sha256sum` by running:
```sh
# x86_64
sha256sum ./nym-vpn-cli_0.1.0_macos_x86_64.zip

# aarch64
sha256sum ./nym-vpn-cli_0.1.0_macos_aarch64.zip
```
* Compare the result with the sha256 hash shared on the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2)
* Extract files with `unzip` command or manually as you are used to
```sh
# for CLI
chmod +x ./nym-vpn-cli
```

* Create Sandbox environment config file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `sandbox.env` in the same directory as your NymVPN binaries. In case of GUI setup, see the steps below.

### GUI Installation

We created a [script](https://gist.github.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19) which does download, extraction, installation and configuration for MacOS users automatically following the steps below:

* To download the script, open a terminal in a directory where you want to download the script and run:
```sh
curl -o nym-vpn-client-executor.sh - L https://gist.githubusercontent.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19/raw/4397365b4cf74594c7f99c1ef5d690b2f5b41192/nym-vpn-client-macos-executor.sh
```
* Make executable
```sh
chmod u+x nym-vpn-client-macos-executor.sh
```
* Run
```sh
./nym-vpn-client-macos-executor.sh
```
* When prompted to verify `sha256sum` paste one from the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) including the binary name (all as one input with a space in between), for example:
```sh
06c7c82f032f230187da1002a9a9a88242d3bbf6c5c09dc961a71df151d768d0  nym-vpn-ui_0.0.2_macos_x86_64.zip
```
* The script will run the application and it will prompt you for a country code to exit, chose one of the offered options

In case of errors check out the [troubleshooting](./nym-vpn-troubleshooting.html#installing-gui-on-macos-not-working) section.

## Run NymVPN

* **For NymVPN to work, all other VPNs must be switched off!**
* At this alpha stage of NymVPN, network connection (wifi) must be re-connected after or in-between NymVPN testing rounds.

### Run CLI

Make sure your terminal is open in the same directory as your `nym-vpn-cli` binary.

* Find the entire command with all the needed arguments' values and your wireguard private key for testing purposes at [nymvpn.com/en/alpha](https://nymvpn.com/en/alpha)
* Run it with `sudo` as root, the command will look like this with specified arguments:
```sh
sudo ./nym-vpn-cli -c ./sandbox.env --entry-gateway-id <ENTRY_GATEWAY_ID> --exit-router-address <EXIT_ROUTER_ADDRESS> --enable-wireguard --private-key <PRIVATE_KEY> --wg-ip <WG_IP>
```
* To chose different Gateways, visit [nymvpn.com/en/alpha/api/gateways](https://nymvpn.com/en/alpha/api/gateways)
* To see all possibilities see [command explanation](#cli-commands-and-options) below

### Run GUI

Make sure you went through the GUI configuration in the [preparation section](#gui-installation).

You may be able to have a NymVPN application icon in your app menu. However this may not work as the application needs root permission.

* Run GUI from terminal:
```sh
sudo $nym_vpn_dir/nym-vpn
```

In case of errors, see [troubleshooting section](./nym-vpn-troubleshooting.md#macos-alert-on-nymvpn-ui-startup).

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
Usage: nym-vpn-cli [OPTIONS]

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file describing the network
      --mixnet-client-path <MIXNET_CLIENT_PATH>
          Path to the data directory of a previously initialised mixnet client, where the keys reside
      --entry-gateway-id <ENTRY_GATEWAY_ID>
          Mixnet public ID of the entry gateway
      --entry-gateway-country <ENTRY_GATEWAY_COUNTRY>
          Auto-select entry gateway by country ISO
      --exit-router-address <EXIT_ROUTER_ADDRESS>
          Mixnet recipient address
      --exit-gateway-id <EXIT_GATEWAY_ID>

      --exit-router-country <EXIT_ROUTER_COUNTRY>
          Mixnet recipient address
      --enable-wireguard
          Enable the wireguard traffic between the client and the entry gateway
      --private-key <PRIVATE_KEY>
          Associated private key
      --wg-ip <WG_IP>
          The IP address of the wireguard interface
      --ip <IP>
          The IP address of the TUN device
      --mtu <MTU>
          The MTU of the TUN device
      --disable-routing
          Disable routing all traffic through the VPN TUN device
      --enable-two-hop
          Enable two-hop mixnet traffic. This means that traffic jumps directly from entry gateway to exit gateway
      --enable-poisson-rate
          Enable Poisson process rate limiting of outbound traffic
  -h, --help
          Print help
  -V, --version
          Print version


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
