# Maintenance

 <!---
TODO
- [ ] Compare mixnode, gateway and NR steps of upgrading and automation and make a generic page - this one - for all of them with additional notes for particular nodes
    - mixnode and gateway done, NR left
--->
## Upgrading your node

> The process is the similar for mixnode, gateway and network requester. In the following steps we use a placeholder `<NODE>` in the commands, please change it for the type of node you want to upgrade. Any particularities for the given type of node are included.

Upgrading your node is a two-step process:
* Updating the binary and `~/.nym/<NODE>/<YOUR_ID>/config.toml` on your VPS
* Updating the node information in the [mixnet smart contract](https://nymtech.net/docs/nyx/mixnet-contract.html). **This is the information that is present on the [mixnet explorer](https://explorer.nymtech.net)**.

### Step 1: Upgrading your binary
Follow these steps to upgrade your mix node binary and update its config file:
* pause your mix node process.
* replace the existing binary with the newest binary (which you can either [compile yourself](https://nymtech.net/docs/binaries/building-nym.html) or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* re-run `init` with the same values as you used initially. **This will just update the config file, it will not overwrite existing keys**.
* restart your mix node process with the new binary.

### Step 2: Updating your node information in the smart contract
Follow these steps to update the information about your `<NODE>` which is publicly available from the [Nym API](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

### Updating node information via the Desktop Wallet
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:  

![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page and click `Submit changes to the blockchain`.  

![Node Settings Page](../images/wallet-screenshots/node_settings.png)

### Updating node information via the CLI
If you want to bond your `<NODE>` via the CLI, then check out the [relevant section in the Nym CLI](../../documentation/docs/src/tools/nym-cli.md#upgrade-a-mix-node) docs.


## VPS Setup and Automation
### Configure your firewall
Although your `<NODE>` is now ready to recieve traffic, your server may not be. The following commands will allow you to set up a firewall using `ufw`.

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

Finally open your `<NODE>` p2p port, as well as ports for ssh and ports for verloc and measurement pings:

```
# for mixnode
sudo ufw allow 1789,1790,8000,22/tcp

# for gateway
sudo ufw allow 1789,22,9000/tcp

# check the status of the firewall
sudo ufw status
```

For more information about your mix node's port configuration, check the [mix node port reference table](./mix-node-setup.md#mixnode-port-reference) or [gateway port reference table](https://nymtech.net/docs/nodes/gateway-setup.html#gateway-port-reference) below.

### Automating your node with tmux and systemd

Although it’s not totally necessary, it's useful to have the mix node automatically start at system boot time. 

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

No when you installed tmux on your VPS, let's run a mixnode on tmux, which allows you to detach your terminal and let your `<NODE>` run on its own on the VPS.

* Pause your `<NODE>`
* Start tmux with the command 
```
tmux
```
* The tmux terminal should open in the same working directory, just the layout changed into tmux default layout.
* Start the `<NODE>` again with a command:
```
./<NODE> run --id <YOUR_ID>
```
* Now, without closing the tmux window, you can close the whole terminal and the `<NODE>` (and any other process running in tmux) will stay active.
* Next time just start your teminal, ssh into the VPS and run the following command to attach back to your previous session:
```
tmux attach-session
```
* To see keybinding options of tmux press `ctrl`+`b` and after 1 second `?`

#### systemd

Here's a systemd service file to do that:

For mixnode:

```ini
[Unit]
Description=Nym Mixnode ({{mix_node_release_version}})
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

For gateway:

```ini
[Unit]
Description=Nym Gateway ({{platform_release_version}})
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

Change the `<PATH>` in `ExecStart` to point at your `<NODE>` binary (`nym-mixnode` or `nym-gateway), and the `<USER>` so it is the user you are running as.

If you have built nym in the `$HOME` directory on your server, and your username is `jetpanther`, then the start command for nym mixnode might look like this:

`ExecStart=/home/jetpanther/nym/target/release/nym-mixnode run --id <YOUR_ID>`. Basically, you want the full `/path/to/nym-mixnode run --id whatever-your-node-id-is`

Then run:

```
# for mixnode
systemctl enable nym-mixnode.service

# for gateway
systemctl enable nym-gateway.service
```

Start your node:

```
# for mixnode
service nym-mixnode start

# for gateway
service nym-gateway start

```

This will cause your node to start at system boot time. If you restart your machine, the node will come back up automatically.

You can also do `service <NODE> stop` or `service <NODE> restart`.

Note: if you make any changes to your systemd script after you've enabled it, you will need to run:

```
systemctl daemon-reload
```

This lets your operating system know it's ok to reload the service configuration.

### Setting the ulimit

Linux machines limit how many open files a user is allowed to have. This is called a `ulimit`.

`ulimit` is 1024 by default on most systems. It needs to be set higher, because mix nodes make and receive a lot of connections to other nodes.

If you see errors such as:

```
Failed to accept incoming connection - Os { code: 24, kind: Other, message: "Too many open files" }
```

This means that the operating system is preventing network connections from being made.

#### Set the ulimit via `systemd` service file

The ulimit setup is relevant for maintenance of nym mixnode only.

Query the `ulimit` of your mix node with:

```
grep -i "open files" /proc/$(ps -A -o pid,cmd|grep nym-mixnode | grep -v grep |head -n 1 | awk '{print $1}')/limits
```

You'll get back the hard and soft limits, which looks something like this:

```
Max open files            65536                65536                files
```

If your output is **the same as above**, your node will not encounter any `ulimit` related issues.

However if either value is `1024`, you must raise the limit via the systemd service file. Add the line:

```
LimitNOFILE=65536
```

Reload the daemon:

```
systemctl daemon-reload
```

or execute this as root for system-wide setting of `ulimit`:

```
echo "DefaultLimitNOFILE=65535" >> /etc/systemd/system.conf
```

Reboot your machine and restart your node. When it comes back, use `cat /proc/$(pidof nym-mixnode)/limits | grep "Max open files"` to make sure the limit has changed to 65535.

#### Set the ulimit on `non-systemd` based distributions

In case you chose tmux option for mixnode automatization, see your `ulimit` list by running:

```
ulimit -a

# watch for the output line -n
-n: file descriptors          1024      
```

You can change it either by running a command:

```
ulimit -u -n 4096
```

or editing `etc/security/conf` and add the following lines:

```
# Example hard limit for max opened files
username        hard nofile 4096

# Example soft limit for max opened files
username        soft nofile 4096
```

Then reboot your server and restart your mixnode.

## Virtual IPs and hosting via Google & AWS
For true internet decentralization we encourage operators to use diverse VPS providers instead of the largest companies offering such services. If for some reasons you have already running AWS or Google and want to setup a `<NODE>` there, please read the following.

On some services (AWS, Google, etc) the machine's available bind address is not the same as the public IP address. In this case, bind `--host` to the local machine address returned by `$(curl ifconfig.me)`, but also specify `--announce-host` with the public IP. Please make sure that you pass the correct, routable `--announce-host`.

For example, on a Google machine, you may see the following output from the `ifconfig` command:

```
ens4: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1460
        inet 10.126.5.7  netmask 255.255.255.255  broadcast 0.0.0.0
        ...
```

The `ens4` interface has the IP `10.126.5.7`. But this isn't the public IP of the machine, it's the IP of the machine on Google's internal network. Google uses virtual routing, so the public IP of this machine is something else, maybe `36.68.243.18`.

`./nym-mixnode init --host 10.126.5.7`, initalises the mix node, but no packets will be routed because `10.126.5.7` is not on the public internet.

Trying `nym-mixnode init --host 36.68.243.18`, you'll get back a startup error saying `AddrNotAvailable`. This is because the mix node doesn't know how to bind to a host that's not in the output of `ifconfig`.

The right thing to do in this situation is to init with a command:
```
./nym-mixnode init --host 10.126.5.7 --announce-host 36.68.243.18
```

This will bind the mix node to the available host `10.126.5.7`, but announce the mix node's public IP to the directory server as `36.68.243.18`. It's up to you as a node operator to ensure that your public and private IPs match up properly.

To find the right IP configuration, contact your VPS provider for support.

## Nym API (previously 'Validator API') endpoints
Numerous API endpoints are documented on the Nym API (previously 'Validator API')'s [Swagger Documentation](https://validator.nymtech.net/api/swagger/index.html). There you can also try out various requests from your browser, and download the response from the API. Swagger will also show you what commands it is running, so that you can run these from an app or from your CLI if you prefer.

### Mix node Reward Estimation API endpoint

The Reward Estimation API endpoint allows mixnode operators to estimate the rewards they could earn for running a Nym mixnode with a specific `MIX_ID`.

> The `<MIX_ID>` can be found in the "Mix ID" column of the [Network Explorer](https://explorer.nymtech.net/network-components/mixnodes/active).

The endpoint is a particularly common for mixnode operators as it can provide an estimate of potential earnings based on factors such as the amount of traffic routed through the mixnode, the quality of the mixnode's performance, and the overall demand for mixnodes in the network. This information can be useful for mixnode operators in deciding whether or not to run a mix node and in optimizing its operations for maximum profitability.

Using this API endpoint returns information about the Reward Estimation:

```
/status/mixnode/<MIX_ID>/reward-estimation
```

Query Response:

```
    "estimation": {
        "total_node_reward": "942035.916721770541325331",
        "operator": "161666.263307386408152071",
        "delegates": "780369.65341438413317326",
        "operating_cost": "54444.444444444444444443"
    },
```

> The unit of value is measured in `uNYM`.

- `estimated_total_node_reward` - An estimate of the total amount of rewards that a particular mix node can expect to receive during the current epoch. This value is calculated by the Nym Validator based on a number of factors, including the current state of the network, the number of mix nodes currently active in the network, and the amount of network traffic being processed by the mix node.

- `estimated_operator_reward` - An estimate of the amount of rewards that a particular mix node operator can expect to receive. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the mix node, the quality of service provided by the mix node, and the operator's stake in the network.

- `estimated_delegators_reward` - An estimate of the amount of rewards that mix node delegators can expect to receive individually. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the mix node, the quality of service provided by the mix node, and the delegator's stake in the network.

- `estimated_node_profit` - An estimate of the profit that a particular mix node operator can expect to earn. This value is calculated by subtracting the mix node operator's `operating_costs` from their `estimated_operator_reward` for the current epoch.

- `estimated_operator_cost` - An estimate of the total cost that a particular mix node operator can expect to incur for their participation. This value is calculated by the Nym Validator based on a number of factors, including the cost of running a mix node, such as server hosting fees, and other expenses associated with operating the mix node.

## Ports
All `<NODE>`-specific port configuration can be found in `$HOME/.nym/<NODE>/<YOUR_ID>/config/config.toml`. If you do edit any port configs, remember to restart your mix node.

### Mix node port reference
| Default port | Use                       |
| ------------ | ------------------------- |
| `1789`       | Listen for mixnet traffic |
| `1790`       | Listen for VerLoc traffic |
| `8000`       | Metrics http API endpoint |


### Gateway port reference
| Default port | Use                       |
|--------------|---------------------------|
| `1789`       | Listen for Mixnet traffic |
| `9000`       | Listen for Client traffic |

/
