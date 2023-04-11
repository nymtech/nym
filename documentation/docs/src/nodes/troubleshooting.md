# Troubleshooting


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

## General Node Config

### Where can I find my private and public keys and  config?

All config and keys files are stored in a directory named after your `id` which you chose during the `init` process, and can be found at the following PATH: `$HOME/.nym/<NODE_TYPE>/<NODE_ID>` where `$HOME` is a home directory of the user (your current user in this case) that launched the node or client.

The directory structure for each node will be roughly as follows:

```
bob@nym:~$ tree /home/nym/.nym/mixnodes/
/home/nym/.nym/mixnodes/
|-- nym010
|   |-- config
|   |   `-- config.toml
|   `-- data
|       |-- private_identity.pem
|       |-- private_sphinx.pem
|       |-- public_identity.pem
|       `-- public_sphinx.pem
```

> If you `cat` the `public_sphinx.pem key`, the output will be different from the public key you will see on Nym [dashboard](https://sandbox-explorer.nymtech.net/). The reason for this is that `.pem` files are encoded in **base64**, however on the web they are in **base58**. Don't be confused if your keys look different. They are the same keys, just with different encoding :)


## Mix Nodes 

### How can I tell my node is up and running and mixing traffic?

First of all check the 'Mixnodes' section of either the Nym Network Explorers: 
* [Mainnet](https://explorer.nymtech.net/overview) 
* [Sandbox testnet](https://sandbox-explorer.nymtech.net/) 

Enter your **identity key** to find your node. Check the contents of the 'mixnode stats' and 'uptime story' sections.

There are 2 community explorers currently, which have been created by [Nodes Guru](https://nodes.guru): 
* [Mainnet](https://mixnet.explorers.guru/)
* [Sandbox testnet](https://sandbox.mixnet.explorers.guru/)

If you want more information, or if your node isn't showing up on the explorer of your choice and you want to double-check, here are some examples on how to check if the node is configured properly.

#### Check from your VPS

Additional details can be obtained via various methods after you connect to your VPS:

##### Socket statistics with `ss`

```
sudo ss -s -t | grep 1789 # if you have specified a different port in your mixnode config, change accordingly
```

This command should return a lot of data containing `ESTAB`. This command should work on every unix based system.

##### List open files and reliant processes with `lsof`

```
# check if lsof is installed:
lsof -v
# install if not installed
sudo apt install lsof
# run against mixnode port
sudo lsof -i TCP:1789 # if you have specified a different port in your mixnode config, change accordingly
```

This command should return something like this:

```
nym-mixno 103349 root   53u  IPv6 1333229972      0t0  TCP [2a03:b0c0:3:d0::ff3:f001]:57844->[2a01:4f9:c011:38ae::5]:1789 (ESTABLISHED)
nym-mixno 103349 root   54u  IPv4 1333229973      0t0  TCP nym:57104->194.5.78.73:1789 (ESTABLISHED)
nym-mixno 103349 root   55u  IPv4 1333229974      0t0  TCP nym:48130->static.236.109.119.168.clients.your-server.de:1789 (ESTABLISHED)
nym-mixno 103349 root   56u  IPv4 1333229975      0t0  TCP nym:52548->vmi572614.contaboserver.net:1789 (ESTABLISHED)
nym-mixno 103349 root   57u  IPv6 1333229976      0t0  TCP [2a03:b0c0:3:d0::ff3:f001]:43244->[2600:1f18:1031:2401:c04b:2f25:ca79:fef3]:1789 (ESTABLISHED)
```

##### Query `systemd` journal with `journalctl`

```
sudo journalctl -u nym-mixnode -o cat | grep "Since startup mixed"
```

If you have created `nym-mixnode.service` file (i.e. you are running your mixnode via `systemd`) then this command shows you how many packets have you mixed so far, and should return a list of messages like this:

```
2021-05-18T12:35:24.057Z INFO  nym_mixnode::node::metrics                      > Since startup mixed 233639 packets!
2021-05-18T12:38:02.178Z INFO  nym_mixnode::node::metrics                      > Since startup mixed 233739 packets!
2021-05-18T12:40:32.344Z INFO  nym_mixnode::node::metrics                      > Since startup mixed 233837 packets!
2021-05-18T12:46:08.549Z INFO  nym_mixnode::node::metrics                      > Since startup mixed 234081 packets!
2021-05-18T12:56:57.129Z INFO  nym_mixnode::node::metrics                      > Since startup mixed 234491 packets!
```

You can add ` | tail` to the end of the command to watch for new entries in real time if needed.

#### Check from your local machine

##### Scan ports with `nmap`:

```
nmap -p 1789 <IP ADDRESS> -Pn
```

If your mixnode is configured properly it should output something like this:

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

You can query gateways by replacing `mixnodes` with `gateways` in the above command, and can query for the mixnodes and gatways on the Sandbox testnet by replacing `validator` with `sandbox-validator`. 


#### Check with Network API

We currently have an API set up returning our metrics tests of the network. There are two endpoints to ping for information about your mixnode, `report` and `history`. Find more information about this in the [Mixnodes metrics documentation](../nodes/mix-node-setup.md#metrics--api-endpoints).

### Why is my node not mixing any packets?

If you are still unable to see your node on the dashboard, or your node is declaring it has not mixed any packets, there are several potential issues:

- The firewall on your host machine is not configured properly.
- You provided incorrect information when bonding your node.
- You are running your mixnode from a VPS without IPv6 support.
- You did not use the `--announce-host` flag while running the mixnode from your local machine behind NAT.
- You did not configure your router firewall while running the mixnode from your local machine behind NAT, or you are lacking IPv6 support.
- Your mixnode is not running at all, it has either exited / panicked or you closed the session without making the node persistent.

```admonish caution
Your mixnode **must speak both IPv4 and IPv6** in order to cooperate with other nodes and route traffic. This is a common reason behind many errors we are seeing among node operators, so check with your provider that your VPS is able to do this!
```

#### Incorrectly configured firewall

The most common reason your mixnode might not be mixing packets is due to a poorly configured firewall. The following commands will allow you to set up a firewall using `ufw`.

```
# check if you have ufw installed
ufw version
# if it is not installed, install with
sudo apt install ufw -y
# enable ufw
sudo ufw enable
# check the status of the firewall
sudo ufw status
```

Finally open your mixnode's p2p port, as well as ports for ssh, http, and https connections, and ports `8000` and `1790` for verloc and measurement pings:

```
sudo ufw allow 1789,1790,8000,22,80,443/tcp
# check the status of the firewall
sudo ufw status
```

#### Incorrect bonding information

Check that you have provided the correct information when bonding your mixnode in the web wallet [interface](https://sandbox-wallet.nymtech.net/). When in doubt, unbond and then rebond your node!

#### Missing `announce-host` flag

On certain cloud providers such as AWS and Google Cloud, you need to do some additional configuration of your firewall and use `--host` with your **local ip** and `--announce-host` with the **public ip** of your mixnode host.

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

Your ISP has to be IPv6 ready if you want to run a mixnode on your local machine. Sadly, in 2020, most of them are not and you won't get an IPv6 address by default from your ISP. Usually it is a extra paid service or they simply don't offer it.

Before you begin, check if you have IPv6 [here](https://test-ipv6.cz/). If not, then don't waste your time to run a node which won't ever be able to mix any packet due to this limitation. Call your ISP and ask for IPv6, there is a plenty of it for everyone!

If all goes well and you have IPv6 available, then you will need to `init` the mixnode with an extra flag, `--announce-host`. You will also need to edit your `config.toml` file each time your IPv4 address changes, that could be a few days or a few weeks.

Additional configuration on your router might also be needed to allow traffic in and out to port 1789 and IPv6 support.

Here is a sample of the `init` command to create the mixnode config.

```
./target/release/nym-mixnode init --id nym-nat --host 0.0.0.0 --announce-host 85.160.12.13 --layer 3
```

- `--host 0.0.0.0` should work everytime even if your local machine IPv4 address changes. For example on Monday your router gives your machine an address `192.168.0.13` and on Wednesday, the DHCP lease will end and you will be asigned `192.168.0.14`. Using `0.0.0.0` should avoid this without having to set any static ip in your router`s configuration.

- you can get your current IPv4 address by either using `curl ipinfo.io` if you're on MacOS or Linux or visiting [whatsmyip site](https://www.whatsmyip.org/). Simply copy it and use it as `--anounce-host` address.

Make sure you check if your node is really mixing. You will need a bit of luck to set this up from your home behind NAT.

### Accidentally killing your node process on exiting session

When you close your current terminal session, you need to make sure you don't kill the mixnode process! There are multiple ways on how to make it persistent even after exiting your ssh session, the easiest solution is to use `nohup`, and the more elegant solution is to run the node with `systemd`.

### Running your mixnode as a background process with `nohup`

`nohup` is a command with which your terminal is told to ignore the `HUP` or 'hangup' signal. This will stop the mixnode process ending if you kill your session.

```
nohup ./nym-mixnode run --id NYM # where `--id NYM` is the id you set during the `init` command.
```

### Running your mixnode as a background process with `systemd`

The most reliable and elegant solution is to create a `systemd.service` file and run the nym-mixnode with `systemctl` command.

Create a file with `nano` at `/etc/systemd/system/nym-mixnode.service` containing the following:

```ini
[Unit]
Description=nym mixnode service
After=network.target

[Service]
Type=simple
User=nym                                      # change as appropriate
LimitNOFILE=65536
ExecStart=/home/nym/nym-mixnode run --id nym  # change as appropriate
KillSignal=SIGINT
Restart=on-failure
RestartSec=30
Restart=on-abort
[Install]
WantedBy=multi-user.target
```

```
# enable the service
sudo systemctl enable nym-mixnode
# start the service
sudo systemctl start nym-mixnode
# check if the service is running properly and mixnode is mixing
sudo systemctl status nym-mixnode
```

Now your node should be mixing all the time, and restart if you reboot your server!

Anytime you change your `systemd` service file you need to `sudo systemctl daemon-reload` in order to restart the service.

### Network configuration seems fine but log still claims `Since startup mixed 0 packets!`

This behavior is most likely caused by a mismatch between your node configuration and the bonding information. Unbond and then rebond your node.

Also make sure to enter all the information in the web wallet exactly as it appears in the log when you start the mixnode process. In particular, the `host` field must contain the _port_ on which your mixnode will listen:

- correct host: `34.12.3.43:1789`
- incorrect host:`34.12.3.43`

### Common errors and warnings

Most of the `ERROR` and `WARN` messages in your node logs are benign - as long as your node outputs `since startup mixed X packets!` in your logs (and this number increases over time), your node is mixing packets. If you want to be sure, check the Nym [dashboard](https://sandbox-explorer.nymtech.net/) or see other ways on how to check if your node is mixing properly as outlined in the section **How can I tell my node is up and running and mixing traffic?** above.

More specific errors and warnings are covered below.

#### `tokio runtime worker` error

If you are running into issues with an error including the following:

```
thread 'tokio-runtime-worker' panicked at 'Failed to create TCP listener: Os { code: 99, kind: AddrNotAvailable, message: "Cannot assign requested address" }'
```

Then you need to `--announce-host <public ip>` and ``--host <local ip>` on startup. This issue arises because of your use of a provider like AWS or Google Cloud, and the fact that your VPS' available bind address is not the same as the public IP address (see [Virtual IPs and hosting via Google and AWS](../nodes/mix-node-setup.md#virtual-ips-and-hosting-via-google--aws) for more information on this issue).

#### `rocket::launch` warnings
These warnings are not an issue, please ignore them. Rocket is a web framework for rust which we are using to provide mixnodes with `/verloc` and `/description` http APIs.

Find more information about this in the [Mixnodes metrics documentation](../nodes/mix-node-setup.md#metrics--api-endpoints).

Rocket runs on port `8000` by default. Although at this stage of the testnet we need Rocket to be reachable via this port, in the future customization of the particular port it uses will be possible.

#### `failed to receive reply to our echo packet within 1.5s. Stopping the test`

This relates to the VerLoc implementation that appeared in `0.10.1`, which has a particularly high log sensitivity. This warning means that the echo packet sent to the mixnode was received, but not sent back. _This will not affect the rate of rewards or performance metrics of your mixnode in the testnet at this point._

#### `Connection to <IP>:1789 seems to be dead`

This warning is normal at the moment, and is _nothing to do with your mixnode!_ It is simply a warning that your node is unable to connect to other peoples' mixnodes for some reason, most likely because they are offline or poorly configured.

### Can I use a port other than 1789 ?

Yes! Here is what you will need to do:

Assuming you would like to use port `1337` for your mixnode, you need to open the new port (and close the old one):

```
sudo ufw allow 1337
sudo ufw deny 1789
```

And then edit the mixnode's config.

> If you want to change the port for an already running node, you need to stop the process before editing your config file.

Assuming your node name is `nym`, the config file is located at `~/.nym/mixnodes/nym/config/config.toml`.

```
nano ~/.nym/mixnodes/nym/config/config.toml
```

You will need to edit two parts of the file. `announce_address` and `listening_address` in the config.toml file. Simply replace `:1789` (the default port) with `:1337` (your new port) after your IP address.

Finally, restart your node. You should see if the mixnode is using the port you have changed in the config.toml file right after you run the node.

### What is `verloc` and do I have to configure my mixnode to implement it?

`verloc` is short for _verifiable location_. Mixnodes and gateways now measure speed-of-light distances to each other, in an attempt to verify how far apart they are. In later releases, this will allow us to algorithmically verify node locations in a non-fakeable and trustworthy manner.

You don't have to do any additional configuration for your node to implement this, it is a passive process that runs in the background of the mixnet from version `0.10.1` onwards.


### Where can I get more help?

The fastest way to reach one of us or get a help from the community, visit our [Telegram help chat](https://t.me/nymchan_help_chat) or head to our [Discord](https://Discord.gg/nym)

For more tech heavy questions join our Keybase channel. Get Keybase [here](https://keybase.io/), then click Teams -> Join a team. Type nymtech.friends into the team name and hit continue. For general chat, hang out in the #general channel. 
