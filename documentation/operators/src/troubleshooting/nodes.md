# Nym Node Troubleshooting

## Binary Build Problems

### I am trying to build from the GitHub archive files and the build fails

GitHub automatically includes .zip and tar.gz files of the Nym repository in its release. You cannot extract these and build - you'll see something like this:

```
  process didn't exit successfully: `/build/nym/src/nym-0.12.1/target/release/build/nym-socks5-client-c1d0f76a8c7d7e9a/build-script-build` (exit status: 101)
  --- stderr
  thread 'main' panicked at 'failed to extract build metadata: could not find repository from '/build/nym/src/nym-0.12.1/clients/socks5'; class=Repository (6); code=NotFound (-3)', clients/socks5/build.rs:7:31
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
error: build failed
```

Why does this happen?

We have scripts which automatically include the Git commit hash and Git tag in the binary for easier debugging later. If you download a .zip and try building from that, it's not a Git repository and build will fail as above.

What to do?

* Open terminal in the directory where you want to have a git repository
* To get Nym repository for the first time, run:
```
git clone https://github.com/nymtech/nym.git
```
* Follow the instructions to build the platform
* To upgrade, pause your nodes, in the same terminal window run `git pull`, follow the upgrade instructions and re-start your nodes.

## General Node Config

### Where can I find my private and public keys and config?

All config and keys files are stored in a directory named after your `id` which you chose during the `init` process, and can be found at the following PATH: `$HOME/.nym/<NODE_TYPE>/<NODE_ID>` where `$HOME` is a home directory of the user (your current user in this case) that launched the node or client.

The directory structure for each node will be roughly as follows:

```
bob@desktop:~/nym/target/release# tree ~/.nym/nym-nodes/
~/.nym/nym-nodes/
└── default-nym-node
    ├── config
    │   └── config.toml
    └── data
        ├── aes128ctr_ipr_ack
        ├── aes128ctr_nr_ack
        ├── clients.sqlite
        ├── cosmos_mnemonic
        ├── description.toml
        ├── ed25519_identity
        ├── ed25519_identity.pub
        ├── ed25519_ipr_identity
        ├── ed25519_ipr_identity.pub
        ├── ed25519_nr_identity
        ├── ed25519_nr_identity.pub
        ├── ipr_gateways_info_store.sqlite
        ├── nr_gateways_info_store.sqlite
        ├── nr_persistent_reply_store.sqlite
        ├── x25519_ipr_dh
        ├── x25519_ipr_dh.pub
        ├── x25519_noise
        ├── x25519_noise.pub
        ├── x25519_nr_dh
        ├── x25519_nr_dh.pub
        ├── x25519_sphinx
        └── x25519_sphinx.pub
```

> If you `cat` the `public_sphinx.pem` key, the output will be different from the public key you will see on Nym [dashboard](https://sandbox-explorer.nymtech.net/). The reason for this is that `.pem` files are encoded in **base64**, however on the web they are in **base58**. Don't be confused if your keys look different. They are the same keys, just with different encoding :)


## Mixnode Mode

### How can I tell my node is up and running and mixing traffic?

First of all check the 'Mixnodes' section of either of the Nym Network Explorers:
* [Mainnet](https://explorer.nymtech.net/)
* [Sandbox testnet](https://sandbox-explorer.nymtech.net/)

Enter your **identity key** to find your node. Check the contents of the `Mixnode stats` and `Routing score` sections.

There are 2 community explorers currently, which have been created by [Nodes Guru](https://nodes.guru):
* [Mainnet](https://mixnet.explorers.guru/)
* [Sandbox testnet](https://sandbox.mixnet.explorers.guru/)

[Here](https://github.com/cosmos/chain-registry/blob/master/nyx/chain.json#L158-L187) is a dictionary with Nyx chain registry entry regarding all explorers.

If you want more information, or if your node isn't showing up on the explorer of your choice and you want to double-check, here are some examples on how to check if the node is configured properly.

#### Check from your VPS

Additional details can be obtained via various methods after you connect to your VPS:

##### Socket statistics with `ss`

```
sudo ss -s -t | grep 1789 # if you have specified a different port in your Mix Node config, change accordingly
```

This command should return a lot of data containing `ESTAB`. This command should work on every unix based system.

##### List open files and reliant processes with `lsof`

```
# check if lsof is installed:
lsof -v
# install if not installed
sudo apt install lsof
# run against nym-mix-node node port
sudo lsof -i TCP:1789 # if you have specified a different port in your mixnode config, change accordingly
```

This command should return something like this:

```
nym-node 103349 root   53u  IPv6 1333229972      0t0  TCP [2a03:b0c0:3:d0::ff3:f001]:57844->[2a01:4f9:c011:38ae::5]:1789 (ESTABLISHED)
nym-node 103349 root   54u  IPv4 1333229973      0t0  TCP nym:57104->194.5.78.73:1789 (ESTABLISHED)
nym-node 103349 root   55u  IPv4 1333229974      0t0  TCP nym:48130->static.236.109.119.168.clients.your-server.de:1789 (ESTABLISHED)
nym-node 103349 root   56u  IPv4 1333229975      0t0  TCP nym:52548->vmi572614.contaboserver.net:1789 (ESTABLISHED)
nym-node 103349 root   57u  IPv6 1333229976      0t0  TCP [2a03:b0c0:3:d0::ff3:f001]:43244->[2600:1f18:1031:2401:c04b:2f25:ca79:fef3]:1789 (ESTABLISHED)
```

##### Query `systemd` journal with `journalctl`

```
sudo journalctl -u nym-node -o cat | grep "Since startup mixed"
```

If you have created `nym-node.service` file (i.e. you are running your Ny, Node via `systemd`) then this command shows you how many packets have you mixed so far, and should return a list of messages like this:

```
2021-05-18T12:35:24.057Z INFO  nym_node::node::metrics                      > Since startup mixed 233639 packets!
2021-05-18T12:38:02.178Z INFO  nym_node::node::metrics                      > Since startup mixed 233739 packets!
2021-05-18T12:40:32.344Z INFO  nym_node::node::metrics                      > Since startup mixed 233837 packets!
2021-05-18T12:46:08.549Z INFO  nym_node::node::metrics                      > Since startup mixed 234081 packets!
2021-05-18T12:56:57.129Z INFO  nym_node::node::metrics                      > Since startup mixed 234491 packets!
```

You can add ` | tail` to the end of the command to watch for new entries in real time if needed.

##### build-info

A `build-info` command prints the build information like commit hash, rust version, binary version just like what command `--version` does. However, you can also specify an `--output=json` flag that will format the whole output as a json, making it an order of magnitude easier to parse.

For example `./target/debug/nym-node --no-banner build-info --output json` will return:

```
{"binary_name":"nym-network-requester","build_timestamp":"2023-07-24T15:38:37.00657Z","build_version":"1.1.23","commit_sha":"c70149400206dce24cf20babb1e64f22202672dd","commit_timestamp":"2023-07-24T14:45:45Z","commit_branch":"feature/simplify-cli-parsing","rustc_version":"1.71.0","rustc_channel":"stable","cargo_profile":"debug"}
```

#### Check from your local machine

##### Scan ports with `nmap`:

```
nmap -p 1789 <IP ADDRESS> -Pn
```

If your Nym Node is configured properly it should output something like this:

```
bob@desktop:~$ nmap -p 1789 95.296.134.220 -Pn

Host is up (0.053s latency).

PORT     STATE SERVICE
1789/tcp open  hello
```

##### Query online nodes:

```
curl --location --request GET 'https://validator.nymtech.net/api/v1/mixnodes/'
```

Will return a list all nodes currently online.

You can query Gateways by replacing `nym-mixnodes` with `nym-gateways` in the above command, and can query for the Mix Nodes and Gateways on the Sandbox testnet by replacing `validator` with `sandbox-validator`.


#### Check with Network API

We currently have an API set up returning our metrics tests of the network. There are two endpoints to ping for information about your Mix Node, `report` and `history`. Find more information about this in the [Mixnodes metrics documentation](../nodes/maintenance.md#metrics--api-endpoints).

### Why is my node not mixing any packets?

If you are still unable to see your node on the dashboard, or your node is declaring it has not mixed any packets, there are several potential issues:

- The firewall on your host machine is not configured properly. Checkout the [instructions](../nodes/vps-setup.md#configure-your-firewall).
- You provided incorrect information when bonding your node.
- You are running your node from a VPS without IPv6 support.
<!-- You did not use the `--announce-host` flag while running the Mix Node from your local machine behind NAT. -->
- You did not configure your router firewall while running the node from your local machine behind NAT, or you are lacking IPv6 support
- Your Mix Node is not running at all, it has either exited / panicked or you closed the session without making the node persistent. Check out the [instructions](../nodes/configuration.md#automating-your-node-with-tmux-and-systemd).

```admonish caution
Your Nym Node **must speak both IPv4 and IPv6** in order to cooperate with other nodes and route traffic. This is a common reason behind many errors we are seeing among node operators, so check with your provider that your VPS is able to do this!
```

#### Incorrect bonding information

Check that you have provided the correct information when bonding your Nym Node in the web wallet interface. When in doubt, un-bond and then re-bond your node!

> All delegated stake will be lost when un-bonding! However the Nym Node must be operational in the first place for the delegation to have any effect.

#### Missing `announce-host` flag

On certain cloud providers such as AWS and Google Cloud, you need to do some additional configuration of your firewall and use `--host` with your **local ip** and `--announce-host` with the **public ip** of your Mix Node host.

If the difference between the two is unclear, contact the help desk of your VPS provider.

#### No IPv6 connectivity

Make sure that your VPS has IPv6 connectivity available with whatever provider you are using.

To get all ip addresses of your host, try following commands:

```
hostname -i
```

Will return your **local ip** address.

```
hostname -I
```

Will return all of the ip addresses of your host. This output should look something like this:

```
bob@nym:~$ hostname -I
88.36.11.23 172.18.0.1 2a01:28:ca:102::1:641
```

- The first **ipv4** is the public ip you need to use for the `--announce-host` flag.
- The second **ipv4** is the local ip you need to use for the `--host` flag.
- The 3rd output should confirm if your machine has ipv6 available.

### Running on a local machine behind NAT with no fixed IP address

Your ISP has to be IPv6 ready if you want to run a Nym Node on your local machine. Sadly, in 2020, most of them are not and you won't get an IPv6 address by default from your ISP. Usually it is an extra paid service or they simply don't offer it.

Before you begin, check if you have IPv6 [here](https://test-ipv6.cz/) or by running command explained in the [section above](#no-ipv6-connectivity). If not, then don't waste your time to run a node which won't ever be able to mix any packet due to this limitation. Call your ISP and ask for IPv6, there is a plenty of it for everyone!

If all goes well and you have IPv6 available, then you will need to `init` the Nym Node with an extra flag, `--announce-host`. You will also need to edit your `config.toml` file each time your IPv4 address changes, that could be a few days or a few weeks. Check the your IPv4 in the [section above](#no-ipv6-connectivity).

Additional configuration on your router might also be needed to allow traffic in and out to port 1789 and IPv6 support.


- `--host 0.0.0.0` should work every time even if your local machine IPv4 address changes. For example on Monday your router gives your machine an address `192.168.0.13` and on Wednesday, the [DHCP](https://en.wikipedia.org/wiki/Dynamic_Host_Configuration_Protocol) lease will end and you will be assigned `192.168.0.14`. Using `0.0.0.0` should avoid this without having to set any static IP in your router's configuration.

- you can get your current IPv4 address by either using `curl ipinfo.io` if you're on MacOS or Linux or visiting [whatsmyip site](https://www.whatsmyip.org/). Simply copy it and use it as `--anounce-host` address.

Make sure you check if your node is really mixing. We are aiming to improve the setup for operators running locally, however you may need a bit of patience to set this up from your home behind NAT.

### Accidentally killing your node process on exiting session

When you close your current terminal session, you need to make sure you don't kill the Mix Node process! There are multiple ways on how to make it persistent even after exiting your ssh session, the easiest solution is to use `tmux` or `nohup`, and the more elegant solution is to run the node with `systemd`. Read the automation manual [here](../nodes/configuration.md#automating-your-node-with-tmux-and-systemd).

### Common errors and warnings

Most of the `ERROR` and `WARN` messages in your node logs are benign - as long as your node outputs `since startup mixed X packets!` (`X` bust be > 0) in your logs (and this number increases over time), your node is mixing packets. If you want to be sure, check the Nym [dashboard](https://sandbox-explorer.nymtech.net/) or see other ways on how to check if your node is mixing properly as outlined in the section [**How can I tell my node is up and running and mixing traffic?**](#how-can-i-tell-my-node-is-up-and-running-and-mixing-traffic?) above.

More specific errors and warnings are covered below.

#### `tokio runtime worker` error

If you are running into issues with an error including the following:

```
thread 'tokio-runtime-worker' panicked at 'Failed to create TCP listener: Os { code: 99, kind: AddrNotAvailable, message: "Cannot assign requested address" }'
```

Then you need to `--announce-host <PUBLIC_IP>` and `--host <LOCAL_IP>` on startup. This issue is addressed [above](#missing-`announce-host`-flag)

### Can I use a port other than 1789?

Yes! Here is what you will need to do:

Assuming you would like to use port `1337` for your Nym Node, you need to open the new port (and close the old one):

```
sudo ufw allow 1337
sudo ufw deny 1789
```

And then edit the node's `config.toml`.

> If you want to change the port for an already running node, you need to stop the process before editing your config file.

The config file is located at `~/.nym/nym-nodes/<ID>/config/config.toml`.

For example, assuming `<YOUR_ID>` was chosen to be `alice-node`:

```
nano ~/.nym/nym-nodes/alice-node/config/config.toml
```

You will need to edit two parts of the file. `announce_address` and `listening_address` in the `config.toml` file. Simply replace `:1789` (the default port) with `:1337` (your new port) after your IP address.

Finally, restart your node. You should see if the Mix Node is using the port you have changed in the config.toml file right after you run the node.

### What is `verloc` and do I have to configure my Nym Node to implement it?

`verloc` is short for _verifiable location_. Mix Nodes and Gateways now measure speed-of-light distances to each other, in an attempt to verify how far apart they are. In later releases, this will allow us to algorithmically verify node locations in a non-fake-able and trustworthy manner.

You don't have to do any additional configuration for your node to implement this, it is a passive process that runs in the background of the mixnet from version `0.10.1` onward.

## Gateways Mode

### My `exit-gateway` is running but appears offline in the explorer

Let your Gateway run and follow these steps:

1. Check if your [firewall configuration](../nodes/vps-setup.md#configure-your-firewall) is active and if the necessary ports are open / allowed, including the ones for Swagger page and Reversed proxy/WSS if this is your case.
2. See if the Gateway is not on the [list of blacklisted Gateways](https://validator.nymtech.net/api/v1/gateways/blacklisted)
3. If it's blacklisted, check out the [point below](#my-gateway-is-blacklisted)

### My Gateway is blacklisted

Nym API measures performance by routing traffic through the Mixnet. If the average of a Gateway's routing score in past 24h is less than 50%, the Gateway gets blacklisted and remains so until this number is higher than 50%.

In case your Gateway appeared on the [blacklist](https://validator.nymtech.net/api/v1/gateways/blacklisted), it's because there is some flaw in the configuration. The most common sources of problems are:

- Bonding before starting the node/service
- Bonding before opening [the needed ports](../nodes/vps-setup.md#configure-your-firewall)
- VPS restarted without operator having a [systemd automation](../nodes/configuration.md#systemd) or some alert notification flow setup

What to do:

- Make sure your node is running and do not stop it if there is no need
- Open all needed ports
- Wait until your node gets above 50% of performance (average of last 24h) - this will likely take several hours, up to a day. During this time your node is tested by `nym-api` and every positive response picks up your Gateway's routing score.

**Do not restart your Nym Node without reason, your routing score will only get worse!**

### My exit Gateway "is still not online..."

The Nyx chain epoch takes up to 60 min. To prevent the Gateway getting blacklisted, it's essential to start it before the bonding process and let it running. In case it already got [blacklisted](#my-gateway-is-backlisted) check the steps above.

<!-- THIS NEEDS TO BE REWORKED
### When enabling `ip_packet_router` (IPR) I get a `client-core error`

This error tells you that you already have IPR keys in your data storage, to activate them you have two options:

1. Open `~/.nym/nym-nodes/<ID>/config/config.toml` and **set the correct values**
```toml
[ip_packer_router_enabled]
enabled = true

# UNDER [storage_paths] CHANGE
ip_packet_router_config = '~/.nym/nym-nodes/<ID>/config/ip_packet_router_config.toml'
```

2. Or **remove the IPR data storage and initialise a new one** with these commands
```toml
rm -rf ~/.nym/nym-nodes/<ID>/data/ip-packet-router-data

./nym-gateway setup-ip-packet-router --id <ID>
```

### My `ip_packet_router` (IPR) seems to not work

There are a few steps to mitigate problems with IPR:

1. Check out the issue right above regarding the [Exit Gateway config](#when-enabling-ip_packet_router-ipr-i-get-a-client-core-error)
2. Open your browser and checkout the Swagger UI page and see if all the roles are enabled:
```sh
# in case of IP
http://<YOUR_LISTENING_IP_ADDRESS>:8080/api/v1/roles

# in case of hostname domain
https://<YOUR_DOMAIN>/api/v1/roles
```
3. Make sure all your [ports are open](https://nymtech.net/operators/nodes/maintenance.html#configure-your-firewall) properly
4. Make sure to run your Gateway with embedded IPR as root. Either in a root shell with your configs in `/root/.nym/` or with a command `sudo -E` which gives root privileges but looks for user config folder
5. If it's all good in the API but you don't see the right tick/badge in the [Performance testing list](https://nymtech.net/events/fast-and-furious), just wait some time and then try to refresh the page
-->
