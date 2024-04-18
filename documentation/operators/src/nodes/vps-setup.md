# VPS Setup & Configuration

We aim for Nym Mixnet to be reliable and quality base layer of privacy accross the globe, while growing as distributed as possible. It's essential to have a fine tuned machine as a foundation for the nodes to meet the requirements and be rewarded for their work.

```admonish info
A suboptimally configured VPS often results in a non-functional node. To follow these steps carefully will save you time and money later on.
```

## VPS Hardware Specs

You will need to rent a VPS to run your node on. One key reason for this is that your node **must be able to send TCP data using both IPv4 and IPv6** (as other nodes you talk to may use either protocol).

Tor community created a very helpful table called [*Good Bad ISPs*](https://community.torproject.org/relay/community-resources/good-bad-isps/), use that one as a guideline for your choice of ISP for your VPS.

Currently we run [performance testing](../testing/performance.md) events to find out the best optimization. Sphinx packet decryption is CPU-bound, so more fast cores the better throughput.

### `nym-node`

Before we conclude the testing with exact results, these are the rough specs:

| **Hardware** | **Minimum Specification** w
| :---         | ---:                      |
| CPU Cores    | 4                         |
| Memory       | 4 GB RAM                  |
| Storage      | 40 GB                     |
| Connectivity | IPv4, IPv6, TCP/IP, UDP   |
| Bandwidth    | 1Tb                       |
| Port speed   | 1Gbps                     |

### Nyx validator

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

For a `nym-node` or Nyx validator to recieve traffic, you need to open ports on the server. The following commands will allow you to set up a firewall using `ufw`.

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

## Setting the ulimit

Linux machines limit how many open files a user is allowed to have. This is called a `ulimit`.

`ulimit` is 1024 by default on most systems. It needs to be set higher, because Nym Nodes make and receive a lot of connections with each others.

If you see errors such as:

```sh
Failed to accept incoming connection - Os { code: 24, kind: Other, message: "Too many open files" }
```

This means that the operating system is preventing network connections from being made.

### Set the `ulimit` via `systemd` service file

> **Replace `<NODE>` variable with the name of your service, for example `nym-node`** as we migrated from `nym-mixnode`, `nym-gateway` and `nym-network-requester`.

The ulimit setup is relevant for maintenance of Nym Node only.

Query the `ulimit` of your `<NODE>` with:

```sh
# for nym-node
grep -i "open files" /proc/$(ps -A -o pid,cmd|grep <NODE> | grep -v grep |head -n 1 | awk '{print $1}')/limits

# for nyx validator
grep -i "open files" /proc/$(ps -A -o pid,cmd|grep nymd | grep -v grep |head -n 1 | awk '{print $1}')/limits
```

You'll get back the hard and soft limits, which looks something like this:

```sh
Max open files            65536                65536                files
```

If your output is **the same as above**, your node will not encounter any `ulimit` related issues.

However if either value is `1024`, you must raise the limit via the systemd service file. Add the line:

```sh
LimitNOFILE=65536
```

Reload the daemon:

```sh
systemctl daemon-reload
```

or execute this as root for system-wide setting of `ulimit`:

```sh
echo "DefaultLimitNOFILE=65535" >> /etc/systemd/system.conf
```

Reboot your server, and restart your node. When it comes back, use:
```sh
# for nym-node
cat /proc/$(pidof <NODE>)/limits | grep "Max open files"

# for validator
cat /proc/$(pidof nym-validator)/limits | grep "Max open files"
```
Make sure the limit has changed to `65535`.

### Set the ulimit on `non-systemd` based distributions

In case you chose tmux option for Nym Node automation, see your `ulimit` list by running:

```sh
ulimit -a

# watch for the output line -n
-n: file descriptors          1024
```

You can change it either by running a command:

```sh
ulimit -u -n 4096
```

or editing `etc/security/conf` and add the following lines:

```sh
# Example hard limit for max opened files
username        hard nofile 4096

# Example soft limit for max opened files
username        soft nofile 4096
```

Then reboot your server and restart your node.


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

| Default port    | Use                           |
|-----------------|-------------------------------|
| `1789`          | Listen for Mixnet traffic     |
| `9000`          | Listen for Client traffic     |
| `9001`          | WSS                           |
| `8080, 80, 443` | Reversed Proxy & Swagger page |

#### Embedded Network Requester functionality ports

| Default port | Use                       |
|--------------|---------------------------|
| `9000`       | Listen for Client traffic |

### Validator port reference

All validator-specific port configuration can be found in `$HOME/.nymd/config/config.toml`. If you do edit any port configs, remember to restart your validator.

| Default port | Use                                  |
|--------------|--------------------------------------|
| `1317`         | REST API server endpoint             |
| `26656`        | Listen for incoming peer connections |
| `26660`        | Listen for Prometheus connections    |
