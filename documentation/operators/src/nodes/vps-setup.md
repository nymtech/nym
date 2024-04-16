# VPS Setup & Configuration

We aim for Nym Mixnet to be reliable and quality base layer of privacy accross the globe, while growing as distributed as possible. It's essential to have a fine tuned machine as a foundation for the nodes to meet the requirements and be rewarded for their work.

```admonish info
A suboptimally configured VPS often results in a non-functional node. To follow these steps carefully will save you time and money later on.
```

## VPS Hardware Specs

You will need to rent a VPS to run your node on. One key reason for this is that your node **must be able to send TCP data using both IPv4 and IPv6** (as other nodes you talk to may use either protocol).

Currently we run [performance testing](../testing/performance.md) events to find out the best optimization. Sphinx packet decryption is CPU-bound, so more fast cores the better throughput.

### `nym-node`

Before we conclude the testing with exact results, these are the rough specs:

| **Hardware** | **Minimum Specification** |
| :---         | ---:                      |
| CPU Cores    | 4                         |
| Memory       | 4 GB RAM                  |
| Storage      | 40 GB                     |
| Connectivity | IPv4, IPv6, TCP/IP, UDP   |
| Bandwidth    | 1Tb                       |
| Port speed   | 1Gbps                     |

### `nym-validator`

The specification mentioned below is for running a full node alongside the nym-api. It is recommended to run `nym-api` and a full Nyx node on the same machine for optimum performance.

Bear in mind that credential signing is primarily CPU-bound, so choose the fastest CPU available to you.

#### Minimum Requirements

| Hardware | **Minimum Specification**                      |
|----------|--------------------------------------------|
| CPU      | 8-cores, 2.8GHz base clock speed or higher |
| RAM      | 16GB DDR4+                                 |
| Disk     | 500 GiB+ NVMe SSD                          |

#### Recommended Requirements

| Hardware | **Minimum Specification**                       |
|----------|---------------------------------------------|
| CPU      | 16-cores, 2.8GHz base clock speed or higher |
| RAM      | 32GB DDR4+                                  |
| Disk     | 1 TiB+ NVMe SSD                             |


#### Full node configuration (validator)

To install a full node from scratch, refer to the [validator setup guide](validator-setup.md) and follow the steps outlined there.

## VPS Configuration

Before node or validator setup, the VPS needs to be configured and tested, to verify your connectivity and make sure that your provider wasn't dishonest with the offered services.

### Configure your Firewall

For a `nym-node` or `nym-validator` to recieve traffic, you need to open ports on the server. The following commands will allow you to set up a firewall using `ufw`.

1. Check `ufw`:
```sh
# check if you have ufw installed
ufw version

# if it is not installed, install with
sudo apt install ufw -y

# enable ufw
sudo ufw enable

# check the status of the firewall
sudo ufw status
```

2. Open all needed ports to have your firewall working correctly:
```sh
# for nym-node
sudo ufw allow 1789,1790,8000,9000,9001,22/tcp

# in case of planning to setup a WSS (for Gateway functionality)
sudo ufw allow 9001/tcp

# inn case of reverse proxy for the swagger page (for Gateway optionality)
sudo ufw allow 8080,80,443

# for validator
sudo ufw allow 1317,26656,26660,22,80,443/tcp
```

3. Check the status of the firewall:
```sh
sudo ufw status
```

For more information about your node's port configuration, check the [port reference table](#ports-reference-table) below.

## Connectivity Test and Configuration

With embedded Network Requester and IP Packet Router, modules routing data for the Mixnet and NymVPN traffic, there are more connectivity requirements on `nym-node` VPS. While we're working on Rust implementation to have these settings as a part of the binary build, in the meantime we wrote two scripts [`nym_network_diagnostics.sh`](https://gist.github.com/tommyv1987/a5fb30f5966e9d7bfbce58d88a85c0c1) and [`enable_networking_for_nym_nodes.sh`](https://gist.github.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77) to support the operators to configure their servers.

1. Download `nym_network_diagnostics.sh`, make executable and run:

```sh
curl -s -L -o gateway_network_check.sh https://gist.githubusercontent.com/tommyv1987/a5fb30f5966e9d7bfbce58d88a85c0c1/raw/27acbdbeecf9e04a0faee6a96e717aa7231935ef/nym_network_diagnostics.sh && chmod u+x gateway_network_check.sh  && ./gateway_network_check.sh
```
~~~admonish example collapsible=true title="An overview of gateway_network_check.sh flow"
```sh
. check ipv4 forwarding status: displays whether ipv4 packet forwarding is enabled on the system.
. check ipv6 forwarding status: shows the status of ipv6 packet forwarding.
. check ufw firewall status: if the ufw (uncomplicated firewall) command is available, it prints the verbose status of the ufw firewall. if ufw is not found, it informs the user.
. identify default network device: finds and displays the default network interface used for internet connectivity. if not found, it reports an error and exits.
. inspect ipv4 firewall rules: lists ipv4 firewall rules that are relevant to forwarding and specifically checks for rules involving "nymtun0" or related to ufw's reject rules for forwarding.
. inspect ipv6 firewall rules: similar to ipv4, but for ipv6 firewall rules, checking for forwarding rules that involve "nymtun0" or are related to ufw's reject rules.
. examine ipv4 routing table: prints the current ipv4 routing table, showing how ipv4 traffic is routed on the device.
. examine ipv6 routing table: shows the ipv6 routing table, detailing the routing of ipv6 traffic.
. check ipv4 connectivity: performs a ping test to google.com with ipv4 to verify internet connectivity.
. check ipv6 connectivity: similar to the ipv4 check but uses ipv6 to ping google.com, verifying ipv6 internet access.
. check internet and mixnet connectivity (ipv4) via nymtun0: tests ipv4 connectivity through the "nymtun0" interface by fetching a joke from an online api. if a joke is returned, it indicates successful connectivity.
. check internet and mixnet connectivity (ipv6) via nymtun0: tests ipv6 connectivity through "nymtun0". if no globally routable ipv6 address is found, it advises on potential actions. if an address is found, it attempts to fetch a joke to confirm connectivity.
```
~~~

2. Download `enable_network_diagnostics.sh`, make executable and run:

```sh
curl -s -L -o https://gist.githubusercontent.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77/raw/7adf0d06d83561598c908e29b4a715c11f6432bf/enable_networking_for_nym_nodes.sh && chmod u+x enable_networking_for_nym_nodes.sh && sudo ./enable_networking_for_nym_nodes.sh
```
~~~admonish example collapsible=true title="An overview of enable_network_diagnostics.sh flow"
```sh
overview:
script usage guide & function invocation: offers a command-line interface for executing specific script functions based on user input, providing flexibility in network configuration and diagnostics.

logic flow:

- it finds the default network device and set up tunnel_interface: identifies the internet-facing interface and sets a variable for a specified tunnel interface.
- install persistent iptables: updates the system's package list and installs the iptables-persistent package, ensuring firewall rules are maintained after reboot.

the functions:

- adjust ipv6 forwarding: modifies sysctl.conf to ensure ipv6 forwarding is enabled, making changes as necessary based on the current configuration.
- apply iptables ipv4 rule: establishes nat (network address translation) and forwarding rules for ipv4 traffic, facilitating packet forwarding between the tunnel interface and the internet.
- apply iptables ipv6 rule: sets up ipv6 forwarding rules and enables ipv6 packet forwarding, calling the function to adjust ipv6 forwarding settings.
- remove iptables ipv4 rule: removes previously set nat and forwarding rules for ipv4, reverting changes made to iptables.
- remove iptables ipv6 rule: clears ipv6 forwarding rules and disables ipv6 packet forwarding, undoing modifications to ip6tables.
- check ipv6 & ipv4 forwarding status: verifies whether packet forwarding for both ipv4 and ipv6 is active, reporting the status.
- inspect nymtun iptables rules: outputs current ipv4 and ipv6 firewall rules, focusing particularly on those affecting the tunnel interface.
- test internet & mixnet connectivity via nymtun (ipv4 & ipv6): attempts to confirm connectivity through the tunnel interface by fetching jokes over ipv4 and ipv6, serving as a practical connectivity test.
- apply all iptables rules for nymtun: executes functions to set iptables rules for both ipv4 and ipv6, configuring the system for packet forwarding.
- remove all iptables rules for ipv6 & ipv4: executes functions to clear all iptables modifications related to packet forwarding.
-

summary:
this is a comprehensive script for configuring network packet forwarding and iptables rules,
aimed at ensuring smooth operation of a tunnel interface.
it includes functionality for both setup and tear-down of nymtun network configurations,
alongside diagnostics for verifying system settings and network connectivity.
```
~~~
  - The process may prompt you if you want to save current IPv4 rules, choose yes.
![](../images/ip_table_prompt.png)

If all the setup went smooth, your server is ready to connect `nym-node` with the rest of the Mixnet. There are a few good configuration suggestions, especially to be considered for Gateway functionality, like Web Secure Socket or Reversed Proxy setup. Visit [Proxy configuration](proxy-configuration) page to see the guides.

## Ports reference tables

All node-specific port configuration can be found in `$HOME/.nym/<NODE>/<YOUR_ID>/config/config.toml`. If you do edit any port configs, remember to restart your client and node processes.

### Nym node port reference

#### Mix Node functionality ports

| Default port | Use                       |
| ------------ | ------------------------- |
| `1789`       | Listen for Mixnet traffic |
| `1790`       | Listen for VerLoc traffic |
| `8000`       | Metrics http API endpoint |

#### Gateway functionality ports

| Default port | Use                       |
|--------------|---------------------------|
| `1789`       | Listen for Mixnet traffic |
| `9000`       | Listen for Client traffic |
| `9001`       | WSS                       |

#### Embedded Network Requester functionality ports

| Default port | Use                       |
|--------------|---------------------------|
| `9000`       | Listen for Client traffic |

### Validator port reference

All validator-specific port configuration can be found in `$HOME/.nymd/config/config.toml`. If you do edit any port configs, remember to restart your validator.

| Default port | Use                                  |
|--------------|--------------------------------------|
| 1317         | REST API server endpoint             |
| 26656        | Listen for incoming peer connections |
| 26660        | Listen for Prometheus connections    |
