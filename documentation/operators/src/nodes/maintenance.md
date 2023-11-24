# Maintenance

## Useful commands

> Adding `--no-banner` startup flag will prevent Nym banner being printed even if run in tty environment.

**build-info**

A `build-info` command prints the build information like commit hash, rust version, binary version just like what command `--version` does. However, you can also specify an `--output=json` flag that will format the whole output as a json, making it an order of magnitude easier to parse.

For example `./target/debug/nym-network-requester --no-banner build-info --output json` will return:

```sh
{"binary_name":"nym-network-requester","build_timestamp":"2023-07-24T15:38:37.00657Z","build_version":"1.1.23","commit_sha":"c70149400206dce24cf20babb1e64f22202672dd","commit_timestamp":"2023-07-24T14:45:45Z","commit_branch":"feature/simplify-cli-parsing","rustc_version":"1.71.0","rustc_channel":"stable","cargo_profile":"debug"}
```

## Upgrading your node

> The process is the similar for Mix Node, Gateway and Network Requester. In the following steps we use a placeholder `<NODE>` in the commands, please change it for the type of node you want to upgrade. Any particularities for the given type of node are included.

Upgrading your node is a two-step process:
* Updating the binary and `~/.nym/<NODE>/<YOUR_ID>/config/config.toml` on your VPS
* Updating the node information in the [mixnet smart contract](https://nymtech.net/docs/nyx/mixnet-contract.html). **This is the information that is present on the [mixnet explorer](https://explorer.nymtech.net)**.

### Step 1: Upgrading your binary
Follow these steps to upgrade your Node binary and update its config file:
* Pause your node process.
    - if you see the terminal window with your node, press `ctrl + c`
    - if you run it as `systemd` service, run: `systemctl stop nym-<NODE>.service`
* Replace the existing `<NODE>` binary with the newest binary (which you can either [compile yourself](https://nymtech.net/docs/binaries/building-nym.html) or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* Re-run `init` with the same values as you used initially for your `<NODE>` ([Mix Node](./mix-node-setup.md#initialising-your-mix-node), [Gateway](./gateway-setup.md#initialising-your-gateway)) . **This will just update the config file, it will not overwrite existing keys**.
* Restart your node process with the new binary:
    - if your node is not automated, just `run` your `<NODE>` with `./nym-<NODE> run --id <ID>`. Here are exact guidelines for [Mix Node](./mix-node-setup.md#running-your-mix-node) and [Gateway](./gateway-setup.md#running-your-gateway). 
    - if you automatized your node via systemd (recommended) run:  
```sh
systemctl daemon-reload # to pickup the new unit file
systemctl start nym-<NODE>.service
journalctl -f -u <NODE>.service # to monitor log of you node
```

If these steps are too difficult and you prefer to just run a script, you can use [ExploreNYM script](https://github.com/ExploreNYM/bash-tool) or one done by [Nym developers](https://gist.github.com/tommyv1987/4dca7cc175b70742c9ecb3d072eb8539).

> In case of a Network Requester this is all, the following step is only for Mix Nodes and Gateways.

### Step 2: Updating your node information in the smart contract
Follow these steps to update the information about your `<NODE>` which is publicly available from the [`nym-api`](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [Mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

### Updating node information via the Desktop Wallet (recommended)
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:  

![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page and click `Submit changes to the blockchain`.  

![Node Settings Page](../images/wallet-screenshots/node_settings.png)

### Updating node information via the CLI
If you want to bond your `<NODE>` via the CLI, then check out the [relevant section in the Nym CLI](https://nymtech.net/docs/tools/nym-cli.html#upgrade-a-mix-node) docs.


### Upgrading Network Requester to >= v1.1.10 from <v1.1.9

In the previous version of the network-requester, users were required to run a nym-client along side it to function. As of `v1.1.10`, the network-requester now has a nym client embedded into the binary, so it can run standalone.

If you are running an existing Network Requester registered with nym-connect, upgrading requires you move your old keys over to the new Network Requester configuration. We suggest following these instructions carefully to ensure a smooth transition.

Initiate the new Network Requester:

```sh
nym-network-requester init --id <YOUR_ID>
```

Copy the old keys from your client to the network-requester configuration that was created above:

```sh
cp -vr ~/.nym/clients/myoldclient/data/* ~/.nym/service-providers/network-requester/<YOUR_ID>/data
```

Edit the configuration to match what you used on your client. Specifically, edit the configuration file at:

```sh
~/.nym/service-providers/network-requester/<YOUR_ID>/config/config.toml
```

Ensure that the fields `gateway_id`, `gateway_owner`, `gateway_listener` in the new config match those in the old client config at:

```sh
~/.nym/clients/myoldclient/config/config.toml
```

### Upgrading your validator

Upgrading from `v0.31.1` -> `v0.32.0` process is fairly simple. Grab the `v0.32.0` release tarball from the [`nyxd` releases page](https://github.com/nymtech/nyxd/releases), and untar it. Inside are two files:

- the new validator (`nyxd`) v0.32.0
- the new wasmvm (it depends on your platform, but most common filename is `libwasmvm.x86_64.so`)

Wait for the upgrade height to be reached and the chain to halt awaiting upgrade, then:

* copy `libwasmvm.x86_64.so` to the default LD_LIBRARY_PATH on your system (on Ubuntu 20.04 this is `/lib/x86_64-linux-gnu/`) replacing your existing file with the same name.
* swap in your new `nyxd` binary and restart.

You can also use something like [Cosmovisor](https://github.com/cosmos/cosmos-sdk/tree/main/tools/cosmovisor) - grab the relevant information from the current upgrade proposal [here](https://nym.explorers.guru/proposal/9).

Note: Cosmovisor will swap the `nyxd` binary, but you'll need to already have the `libwasmvm.x86_64.so` in place.

#### Common reasons for your validator being jailed

The most common reason for your validator being jailed is that your validator is out of memory because of bloated syslogs.

Running the command `df -H` will return the size of the various partitions of your VPS.

If the `/dev/sda` partition is almost full, try pruning some of the `.gz` syslog archives and restart your validator process.


## Run Web Secure Socket (WSS) on Gateway

Now you can run WSS on your Gateway.

### WSS on a new Gateway

These steps are for an operator who is setting up a Gateway for the first time and wants to run it with WSS.

New flags will need to be added to the `init` and `run` command. The `--host` option should be replaced with `--listening-address`, this is the IP address which is used for receiving sphinx packets and listening to client data. 

Another flag `--public-ips` is required. It's a comma separated list of IP’s that are announced to the `nym-api`. In the most cases `--public-ips` **is the address used for bonding.** 

If the operator wishes to run WSS, an optional `--hostname` flag is also required, that can be something like `mainnet-gateway2.nymtech.net`. Make sure to enable all necessary [ports](maintenance.md#configure-your-firewall) on the Gateway. 

The Gateway will then be accessible on something like: *http://85.159.211.99:8080/api/v1/swagger/index.html*

Are you seeing something like: *this node attempted to announce an invalid public address: 0.0.0.0.*? 

Please modify `[host.public_ips]` section of your config file stored as `~/.nym/gateways/<ID>/config/config.toml`.

If so the flags are going to be slightly different:

```
--listening-address 0.0.0.0 --public-ips "$(curl -4 https://ifconfig.me)"
```

### WSS on an existing Gateway

In case you already run a working Gateway and want to add WSS on it, here are the pre-requisites to running WSS on Gateways:

* You need to use the latest `nym-gateway` binary [version](./gateway-setup.md#current-version) and restart it.
* That will add the relevant fields to update your config.
* These two values will be added and need to be amended in your config.toml:

```sh
clients_wss_port = 0
hostname = ""
```

Then you can run this:

```sh
port=$1 // in the example below we will use 9001
host=$2 = // this would be a domain name registered for your Gateway for example: mainnet-gateway2.nymtech.net


sed -i "s/clients_wss_port = 0/clients_wss_port = ${port}/" ${HOME}/.nym/gateways/*/config/config.toml
sed -i "s|hostname = ''|hostname = '${host}'|" ${HOME}/.nym/gateways/*/config/config.toml
```
The following shell script can be run:

```sh
#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Usage: sudo ./install_run_caddy.sh <host_name> <port_to_run_wss>"
    exit 1
fi

host=$1
port_value=$2

apt install -y debian-keyring debian-archive-keyring apt-transport-https
apt --fix-broken install

curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg

curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list

apt update
apt install caddy

systemctl enable caddy.service 

cd /etc/caddy

# check if Caddyfile exists, if it does, remove and insert a new one
if [ -f Caddyfile ]; then
    echo "removing caddyfile inserting a new one"
    rm -f Caddyfile
fi

cat  <<EOF >> Caddyfile 
${host}:${port_value} {
	@websockets {
		header Connection *Upgrade*
		header Upgrade websocket
	}
	reverse_proxy @websockets localhost:9000
}
EOF

cat Caddyfile

echo "script completed successfully!"

systemctl restart caddy.service
echo "have a nice day!"
exit 0

```

Although your Gateway is Now ready to use its `wss_port`, your server may not be ready - the following commands will allow you to set up a properly configured firewall using `ufw`:

```sh
ufw allow 9001/tcp
```

Lastly don't forget to restart your Gateway, now the API will render the WSS details for this Gateway:

## Configure your firewall

Although your `<NODE>` is now ready to receive traffic, your server may not be. The following commands will allow you to set up a firewall using `ufw`.

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

Finally open your `<NODE>` p2p port, as well as ports for ssh and ports for verloc and measurement pings:

```sh
# for Mix Node, Gateway and Network Requester
sudo ufw allow 1789,1790,8000,9000,9001,22/tcp

# in case of setting up WSS on Gateway add:
sudo ufw allow 9001/tcp

# In case of reverse proxy for the Gateway swagger page add:
sudo ufw allow 8080,80/443

# for validator
sudo ufw allow 1317,26656,26660,22,80,443/tcp
```

Check the status of the firewall:
```sh
sudo ufw status
```

For more information about your node's port configuration, check the [port reference table](./maintenance.md#gateway-port-reference) below.

## VPS Setup and Automation

### Automating your node with nohup, tmux and systemd

Although it’s not totally necessary, it's useful to have the Mix Node automatically start at system boot time. 

#### nohup

`nohup` is a command with which your terminal is told to ignore the `HUP` or 'hangup' signal. This will stop the node process ending if you kill your session.

```sh
nohup ./<NODE> run --id <YOUR_ID> # where `<YOUR_ID>` is the id you set during the `init` command and <NODE> depends on which node you starting
```

#### tmux

One way is to use `tmux` shell on top of your current VPS terminal. Tmux is a terminal multiplexer, it allows you to create several terminal windows and panes from a single terminal. Processes started in `tmux` keep running after closing the terminal as long as the given `tmux` window was not terminated.  

Use the following command to get `tmux`.  

Platform|Install Command
---|---
Arch Linux|`pacman -S tmux`
Debian or Ubuntu|`apt install tmux`
Fedora|`dnf install tmux`
RHEL or CentOS|`yum install tmux`
macOS (using Homebrew|`brew install tmux`
macOS (using MacPorts)|`port install tmux`
openSUSE|`zypper install tmux`
  
In case it didn't work for your distribution, see how to build `tmux` from [version control](https://github.com/tmux/tmux#from-version-control).  

**Running tmux**

No when you installed tmux on your VPS, let's run a Mix Node on tmux, which allows you to detach your terminal and let your `<NODE>` run on its own on the VPS.

* Pause your `<NODE>`
* Start tmux with the command 
```sh
tmux
```
* The tmux terminal should open in the same working directory, just the layout changed into tmux default layout.
* Start the `<NODE>` again with a command:
```sh
./<NODE> run --id <YOUR_ID>
```
* Now, without closing the tmux window, you can close the whole terminal and the `<NODE>` (and any other process running in tmux) will stay active.
* Next time just start your teminal, ssh into the VPS and run the following command to attach back to your previous session:
```sh
tmux attach-session
```
* To see keybinding options of tmux press `ctrl`+`b` and after 1 second `?`

#### systemd

Here's a systemd service file to do that:

##### For Mix Node

```ini
[Unit]
Description=Nym Mixnode <VERSION>
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=<USER>
LimitNOFILE=65536
ExecStart=/home/<USER>/<PATH>/nym-mixnode run --id <YOUR_ID>
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

* Put the above file onto your system at `/etc/systemd/system/nym-mixnode.service`.

##### For Gateway

```ini
[Unit]
Description=Nym Gateway <VERSION>
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nym
LimitNOFILE=65536
ExecStart=/home/<USER>/<PATH>/nym-gateway run --id <YOUR_ID>
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

* Put the above file onto your system at `/etc/systemd/system/nym-gateway.service`.

##### For Network Requester

```ini
[Unit]
Description=Nym Network Requester <VERSION>
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nym # replace this with whatever user you wish
LimitNOFILE=65536
# remember to add the `--enable-statistics` flag if running as part of a service grant and check the path to your nym-network-requester binary
ExecStart=/home/nym/nym-network-requester run --id <YOUR_ID>
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Now enable and start your requester:

```sh
systemctl enable nym-network-requester.service
systemctl start nym-network-requester.service

# you can always check your requester has succesfully started with:
systemctl status nym-network-requester.service
```
* Put the above file onto your system at `/etc/systemd/system/nym-network-requester.service`.

##### For Validator

Below is a systemd unit file to place at `/etc/systemd/system/nymd.service`:

```ini
[Unit]
Description=Nyxd
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=<USER>                                                       # change to your user
Type=simple
Environment="LD_LIBRARY_PATH=/home/<USER>/<PATH_TO_NYX_BINARIES>" # change to correct path
ExecStart=/home/<USER>/<PATH_TO_NYX_BINARIES>/nymd start          # change to correct path
Restart=on-failure
RestartSec=30
LimitNOFILE=infinity

[Install]
WantedBy=multi-user.target
```

Proceed to start it with:

```sh
systemctl daemon-reload # to pickup the new unit file
systemctl enable nymd   # to enable the service
systemctl start nymd    # to actually start the service
journalctl -f -u nymd # to monitor system logs showing the service start
```

##### Following steps for Nym Mixnet nodes

Change the `<PATH>` in `ExecStart` to point at your `<NODE>` binary (`nym-mixnode`, `nym-gateway` or `nym-network-requester`), and the `<USER>` so it is the user you are running as.

If you have built nym in the `$HOME` directory on your server, and your username is `jetpanther`, then the start command for nym mixnode might look like this:

`ExecStart=/home/jetpanther/nym/target/release/nym-mixnode run --id <YOUR_ID>`. Basically, you want the full `/path/to/nym-mixnode run --id whatever-your-node-id-is`

Then run:

```sh
systemctl daemon-reload # to pickup the new unit file
```

```sh
# for Mix Node
systemctl enable nym-mixnode.service

# for Gateway
systemctl enable nym-gateway.service
```

Start your node:

```sh
# for Mix Node
service nym-mixnode start

# for Gateway
service nym-gateway start

```

This will cause your node to start at system boot time. If you restart your machine, the node will come back up automatically.

You can monitor system logs of your node by running:
```sh
journalctl -f -u <NODE>.service
# for example journalctl -f -u nym-mixnode.service
```

Or check a status by running:
```sh
systemctl status <NODE>.service
# for example systemctl status nym-mixnode.service
```

You can also do `service <NODE> stop` or `service <NODE> restart`.

Note: if you make any changes to your systemd script after you've enabled it, you will need to run:

```
systemctl daemon-reload
```

This lets your operating system know it's ok to reload the service configuration.

### Setting the ulimit

Linux machines limit how many open files a user is allowed to have. This is called a `ulimit`.

`ulimit` is 1024 by default on most systems. It needs to be set higher, because Mix Nodes make and receive a lot of connections to other nodes.

If you see errors such as:

```sh
Failed to accept incoming connection - Os { code: 24, kind: Other, message: "Too many open files" }
```

This means that the operating system is preventing network connections from being made.

#### Set the `ulimit` via `systemd` service file

> Replace `<NODE>` variable with `nym-mixnode`, `nym-gateway` or `nym-network-requester` according the node you running on your machine.

The ulimit setup is relevant for maintenance of Nym Mix Node only.

Query the `ulimit` of your `<NODE>` with:

```sh
# for nym-mixnode, nym-gateway and nym-network-requester:
grep -i "open files" /proc/$(ps -A -o pid,cmd|grep <NODE> | grep -v grep |head -n 1 | awk '{print $1}')/limits

# for nyx validator:
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

Reboot your machine and restart your node. When it comes back, use:
```sh
# for nym-mixnode, nym-gateway and nym-network-requester:
cat /proc/$(pidof <NODE>)/limits | grep "Max open files"

# for validator
cat /proc/$(pidof nym-validator)/limits | grep "Max open files"
```
Make sure the limit has changed to 65535.

#### Set the ulimit on `non-systemd` based distributions

In case you chose tmux option for Mix Node automation, see your `ulimit` list by running:

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

Then reboot your server and restart your Mix Node.

## Moving a node

In case of a need to move a node from one machine to another and avoiding to lose the delegation, here are few steps how to do it.

The following examples transfers a Mix Node (in case of other nodes, change the `mixnodes` in the command for the `<NODE>` of your desire.

* Pause your node process.

Assuming both machines are remote VPS.

* Make sure your `~/.ssh/<YOUR_KEY>.pub` is in both of the machines `~/.ssh/authorized_keys` file
* Create a `mixnodes` folder in the target VPS. Ssh in from your terminal and run:

```sh
# in case none of the nym configs was created previously
mkdir ~/.nym

#in case no nym Mix Node was initialized previously
mkdir ~/.nym/mixnodes
```
* Move the node data (keys) and config file to the new machine by opening a local terminal (as that one's ssh key is authorized in both of the machines) and running:
```sh
scp -r -3 <SOURCE_USER_NAME>@<SOURCE_HOST_ADDRESS>:~/.nym/mixnodes/<YOUR_ID> <TARGET_USER_NAME>@<TARGET_HOST_ADDRESS>:~/.nym/mixnodes/
```
* Re-run init (remember that init doesn't overwrite existing keys) to generate a config with the new listening address etc.
* Change the node smart contract info via the wallet interface. Otherwise the keys will point to the old IP address in the smart contract, and the node will not be able to be connected, and it will fail up-time checks.
* Re-run the node from the new location. 


## Virtual IPs and hosting via Google & AWS
For true internet decentralization we encourage operators to use diverse VPS providers instead of the largest companies offering such services. If for some reasons you have already running AWS or Google and want to setup a `<NODE>` there, please read the following.

On some services (AWS, Google, etc) the machine's available bind address is not the same as the public IP address. In this case, bind `--host` to the local machine address returned by `$(curl ifconfig.me)`, but also specify `--announce-host` with the public IP. Please make sure that you pass the correct, routable `--announce-host`.

For example, on a Google machine, you may see the following output from the `ifconfig` command:

```sh
ens4: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1460
        inet 10.126.5.7  netmask 255.255.255.255  broadcast 0.0.0.0
        ...
```

The `ens4` interface has the IP `10.126.5.7`. But this isn't the public IP of the machine, it's the IP of the machine on Google's internal network. Google uses virtual routing, so the public IP of this machine is something else, maybe `36.68.243.18`.

`./nym-mixnode init --host 10.126.5.7`, initalises the Mix Node, but no packets will be routed because `10.126.5.7` is not on the public internet.

Trying `nym-mixnode init --host 36.68.243.18`, you'll get back a startup error saying `AddrNotAvailable`. This is because the Mix Node doesn't know how to bind to a host that's not in the output of `ifconfig`.

The right thing to do in this situation is to init with a command:
```sh
./nym-mixnode init --host 10.126.5.7 --announce-host 36.68.243.18
```

This will bind the Mix Node to the available host `10.126.5.7`, but announce the Mix Node's public IP to the directory server as `36.68.243.18`. It's up to you as a node operator to ensure that your public and private IPs match up properly.

To find the right IP configuration, contact your VPS provider for support.

## Nym API (previously 'Validator API') endpoints
Numerous API endpoints are documented on the Nym API (previously 'Validator API')'s [Swagger Documentation](https://validator.nymtech.net/api/swagger/index.html). There you can also try out various requests from your browser, and download the response from the API. Swagger will also show you what commands it is running, so that you can run these from an app or from your CLI if you prefer.

### Mix Node Reward Estimation API endpoint

The Reward Estimation API endpoint allows Mix Node operators to estimate the rewards they could earn for running a Nym Mix Node with a specific `MIX_ID`.

> The `<MIX_ID>` can be found in the "Mix ID" column of the [Network Explorer](https://explorer.nymtech.net/network-components/mixnodes/active).

The endpoint is a particularly common for Mix Node operators as it can provide an estimate of potential earnings based on factors such as the amount of traffic routed through the Mix Node, the quality of the Mix Node's performance, and the overall demand for Mix Nodes in the network. This information can be useful for Mix Node operators in deciding whether or not to run a Mix Node and in optimizing its operations for maximum profitability.

Using this API endpoint returns information about the Reward Estimation:

```sh
/status/mixnode/<MIX_ID>/reward-estimation
```

Query Response:

```sh
    "estimation": {
        "total_node_reward": "942035.916721770541325331",
        "operator": "161666.263307386408152071",
        "delegates": "780369.65341438413317326",
        "operating_cost": "54444.444444444444444443"
    },
```

> The unit of value is measured in `uNYM`.

- `estimated_total_node_reward` - An estimate of the total amount of rewards that a particular Mix Node can expect to receive during the current epoch. This value is calculated by the Nym Validator based on a number of factors, including the current state of the network, the number of Mix Nodes currently active in the network, and the amount of network traffic being processed by the Mix Node.

- `estimated_operator_reward` - An estimate of the amount of rewards that a particular Mix Node operator can expect to receive. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the Mix Node, the quality of service provided by the Mix Node, and the operator's stake in the network.

- `estimated_delegators_reward` - An estimate of the amount of rewards that Mix Node delegators can expect to receive individually. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the Mix Node, the quality of service provided by the Mix Node, and the delegator's stake in the network.

- `estimated_node_profit` - An estimate of the profit that a particular Mix node operator can expect to earn. This value is calculated by subtracting the Mix Node operator's `operating_costs` from their `estimated_operator_reward` for the current epoch.

- `estimated_operator_cost` - An estimate of the total cost that a particular Mix Node operator can expect to incur for their participation. This value is calculated by the Nym Validator based on a number of factors, including the cost of running a Mix Node, such as server hosting fees, and other expenses associated with operating the Mix Node.

### Validator: Installing and configuring nginx for HTTPS
#### Setup
[Nginx](https://www.nginx.com/resources/glossary/nginx/#:~:text=NGINX%20is%20open%20source%20software,%2C%20media%20streaming%2C%20and%20more.&text=In%20addition%20to%20its%20HTTP,%2C%20TCP%2C%20and%20UDP%20servers.) is an open source software used for operating high-performance web servers. It allows us to set up reverse proxying on our validator server to improve performance and security.

Install `nginx` and allow the 'Nginx Full' rule in your firewall:

```sh
sudo ufw allow 'Nginx Full'
```

Check nginx is running via systemctl:

```sh
systemctl status nginx
```

Which should return:

```sh
● nginx.service - A high performance web server and a reverse proxy server
   Loaded: loaded (/lib/systemd/system/nginx.service; enabled; vendor preset: enabled)
   Active: active (running) since Fri 2018-04-20 16:08:19 UTC; 3 days ago
     Docs: man:nginx(8)
 Main PID: 2369 (nginx)
    Tasks: 2 (limit: 1153)
   CGroup: /system.slice/nginx.service
           ├─2369 nginx: master process /usr/sbin/nginx -g daemon on; master_process on;
           └─2380 nginx: worker process
```

#### Configuration

Proxying your validator's port `26657` to nginx port `80` can then be done by creating a file with the following at `/etc/nginx/conf.d/validator.conf`:

```sh
server {
  listen 80;
  listen [::]:80;
  server_name "domain_name";

  location / {
    proxy_pass http://127.0.0.1:26657;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
  }
}
```

Followed by:

```sh
sudo apt install certbot nginx python3
certbot --nginx -d nym-validator.yourdomain.com -m you@yourdomain.com --agree-tos --noninteractive --redirect
```

```admonish caution title=""
If using a VPS running Ubuntu 20: replace `certbot nginx python3` with `python3-certbot-nginx`
```

These commands will get you an https encrypted nginx proxy in front of the API.

### Configuring Prometheus metrics (optional)

Configure Prometheus with the following commands (adapted from NodesGuru's [Agoric setup guide](https://nodes.guru/agoric/setup-guide/en)):

```sh
echo 'export OTEL_EXPORTER_PROMETHEUS_PORT=9464' >> $HOME/.bashrc
source ~/.bashrc
sed -i '/\[telemetry\]/{:a;n;/enabled/s/false/true/;Ta}' $HOME/.nymd/config/app.toml
sed -i "s/prometheus-retention-time = 0/prometheus-retention-time = 60/g" $HOME/.nymd/config/app.toml
sudo ufw allow 9464
echo 'Metrics URL: http://'$(curl -s ifconfig.me)':26660/metrics'
```

Your validator's metrics will be available to you at the returned 'Metrics URL'.

~~~admonish example collapsible=true title="Console output"
```
# HELP go_gc_duration_seconds A summary of the pause duration of garbage collection cycles.
# TYPE go_gc_duration_seconds summary
go_gc_duration_seconds{quantile="0"} 6.7969e-05
go_gc_duration_seconds{quantile="0.25"} 7.864e-05
go_gc_duration_seconds{quantile="0.5"} 8.4591e-05
go_gc_duration_seconds{quantile="0.75"} 0.000115919
go_gc_duration_seconds{quantile="1"} 0.001137591
go_gc_duration_seconds_sum 0.356555301
go_gc_duration_seconds_count 2448
# HELP go_goroutines Number of goroutines that currently exist.
# TYPE go_goroutines gauge
go_goroutines 668
# HELP go_info Information about the Go environment.
# TYPE go_info gauge
go_info{version="go1.15.7"} 1
# HELP go_memstats_alloc_bytes Number of bytes allocated and still in use.
# TYPE go_memstats_alloc_bytes gauge
go_memstats_alloc_bytes 1.62622216e+08
# HELP go_memstats_alloc_bytes_total Total number of bytes allocated, even if freed.
# TYPE go_memstats_alloc_bytes_total counter
go_memstats_alloc_bytes_total 2.09341707264e+11
# HELP go_memstats_buck_hash_sys_bytes Number of bytes used by the profiling bucket hash table.
# TYPE go_memstats_buck_hash_sys_bytes gauge
go_memstats_buck_hash_sys_bytes 5.612319e+06
# HELP go_memstats_frees_total Total number of frees.
# TYPE go_memstats_frees_total counter
go_memstats_frees_total 2.828263344e+09
# HELP go_memstats_gc_cpu_fraction The fraction of this program's available CPU time used by the GC since the program started.
# TYPE go_memstats_gc_cpu_fraction gauge
go_memstats_gc_cpu_fraction 0.03357798610671518
# HELP go_memstats_gc_sys_bytes Number of bytes used for garbage collection system metadata.
# TYPE go_memstats_gc_sys_bytes gauge
go_memstats_gc_sys_bytes 1.3884192e+07
```
~~~

## Ports
All `<NODE>`-specific port configuration can be found in `$HOME/.nym/<NODE>/<YOUR_ID>/config/config.toml`. If you do edit any port configs, remember to restart your client and node processes.

### Mix Node port reference
| Default port | Use                       |
| ------------ | ------------------------- |
| `1789`       | Listen for Mixnet traffic |
| `1790`       | Listen for VerLoc traffic |
| `8000`       | Metrics http API endpoint |


### Gateway port reference
| Default port | Use                       |
|--------------|---------------------------|
| `1789`       | Listen for Mixnet traffic |
| `9000`       | Listen for Client traffic |
| `9001`       | WSS                       |

### Network Requester port reference

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

  
  
