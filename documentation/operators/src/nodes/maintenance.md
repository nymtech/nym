# Maintenance

<!---
TODO
- [ ] Compare mixnode, gateway and NR steps of upgrading and automation and make a generic page - this one - for all of them with additional notes for particular nodes
--->

<!---From mixnode with serinkos edits--->

### Upgrading your mix node

Upgrading your node is a two-step process:
* Updating the binary and `~/.nym/mixnodes/<YOUR_ID>/config.toml` on your VPS
* Updating the node information in the [mixnet smart contract](../nyx/mixnet-contract.md). **This is the information that is present on the [mixnet explorer](https://explorer.nymtech.net)**.

#### Step 1: Upgrading your binary
Follow these steps to upgrade your mix node binary and update its config file:
* pause your mix node process.
* replace the existing binary with the newest binary (which you can either [compile yourself](https://nymtech.net/docs/binaries/building-nym.html) or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* re-run `init` with the same values as you used initially. **This will just update the config file, it will not overwrite existing keys**.
* restart your mix node process with the new binary.

#### Step 2: Updating your node information in the smart contract
Follow these steps to update the information about your mix node which is publically avaliable from the [Nym API](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

#### Updating node information via the Desktop Wallet
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:
\
![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page and click `Submit changes to the blockchain`.
\
![Node Settings Page](../images/wallet-screenshots/node_settings.png)

#### Updating node information via the CLI
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](../../documentation/docs/src/tools/nym-cli.md#upgrade-a-mix-node) docs.

---

## VPS Setup and Automation
### Configure your firewall
The following commands will allow you to set up a firewall using `ufw`.

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

Finally open your mix node's p2p port, as well as ports for ssh and ports `8000` and `1790` for verloc and measurement pings:

```
sudo ufw allow 1789,1790,8000,22/tcp

# check the status of the firewall
sudo ufw status
```

For more information about your mix node's port configuration, check the [mix node port reference table](./mix-node-setup.md#mixnode-port-reference) below.

### Automating your mix node with tmux and systemd

It's useful to have the mix node automatically start at system boot time. 

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

No when you installed tmux on your VPS, let's run a mixnode on tmux, which allows you to detach your terminal and let the mixnode run on its own on the VPS.

* Pause your mixnode
* Start tmux with the command 
```
tmux
```
* The terminal should stay in the same directory, just the layout changed into tmux default layout.
* Start the mixnode again with a command:
```
./nym-mixnode run --id <YOUR_ID>
```
* Now, without closing the tmux window, you can close the whole terminal and the mixnode (and any other process running in tmux) will stay active.
* Next time just start your teminal, ssh into the VPS and run the following command to attach back to your previous session:
```
tmux attach session
```
* To see keybinding options of tmux press `ctrl`+`b` and after 1 second `?`

#### systemd

Here's a systemd service file to do that:

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

Put the above file onto your system at `/etc/systemd/system/nym-mixnode.service`.

Change the `<PATH>` in `ExecStart` to point at your mix node binary (`nym-mixnode`), and the `<USER>` so it is the user you are running as.

If you have built nym in the `$HOME` directory on your server, and your username is `jetpanther`, then the start command might look like this:

`ExecStart=/home/jetpanther/nym/target/release/nym-mixnode run --id your-id`. Basically, you want the full `/path/to/nym-mixnode run --id whatever-your-node-id-is`

Then run:

```
systemctl enable nym-mixnode.service
```

Start your node:

```
service nym-mixnode start
```

This will cause your node to start at system boot time. If you restart your machine, the node will come back up automatically.

You can also do `service nym-mixnode stop` or `service nym-mixnode restart`.

Note: if you make any changes to your systemd script after you've enabled it, you will need to run:

```
systemctl daemon-reload
```

This lets your operating system know it's ok to reload the service configuration.

#### Setting the ulimit

Linux machines limit how many open files a user is allowed to have. This is called a `ulimit`.

`ulimit` is 1024 by default on most systems. It needs to be set higher, because mix nodes make and receive a lot of connections to other nodes.

If you see errors such as:

```
Failed to accept incoming connection - Os { code: 24, kind: Other, message: "Too many open files" }
```

This means that the operating system is preventing network connections from being made.

##### Set the ulimit via `systemd` service file

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

##### Set the ulimit on `non-systemd` based distributions

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



<!---FROM GATEWAY GUIDE - NO EDITS SO FAR--->

#### Step 1: Upgrading your binary
Follow these steps to upgrade your binary and update its config file:
* pause your gateway process.
* replace the existing binary with the newest binary (which you can either compile yourself or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* re-run `init` with the same values as you used initially. **This will just update the config file, it will not overwrite existing keys**.
* restart your gateway process with the new binary.

> Do **not** use the `upgrade` command: there is a known error with the command that will be fixed in a subsequent release.

#### Step 2: Updating your node information in the smart contract
Follow these steps to update the information about your node which is publically avaliable from the [Nym API](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

#### Updating node information via the Desktop Wallet
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:
![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page and click `Submit changes to the blockchain`.
![Node Settings Page](../images/wallet-screenshots/node_settings.png)

#### Updating node information via the CLI
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](../tools/nym-cli.md#upgrade-a-mix-node) docs.

---

## VPS Setup and Automation
### Configure your firewall
Although your gateway is now ready to receive traffic, your server may not be - the following commands will allow you to set up a properly configured firewall using `ufw`:

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

Finally open your gateway's p2p port, as well as ports for ssh and incoming traffic connections:

```
sudo ufw allow 1789,22,9000/tcp
# check the status of the firewall
sudo ufw status
```

For more information about your gateway's port configuration, check the [gateway port reference table](#gateway-port-reference) below.

### Automating your gateway with systemd
Although it's not totally necessary, it's useful to have the gateway automatically start at system boot time. Here's a systemd service file to do that:

```ini
[Unit]
Description=Nym Gateway ({{platform_release_version}})
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nym
LimitNOFILE=65536
ExecStart=/home/nym/nym-gateway run --id supergateway
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Put the above file onto your system at `/etc/systemd/system/nym-gateway.service`.

Change the path in `ExecStart` to point at your gateway binary (`nym-gateway`), and the `User` so it is the user you are running as.

If you have built nym on your server, and your username is `jetpanther`, then the start command might look like this:

`ExecStart=/home/jetpanther/nym/target/release/nym-gateway run --id your-id`. Basically, you want the full `/path/to/nym-gateway run --id whatever-your-node-id-is`


Then run:

```
systemctl enable nym-gateway.service
```

Start your node:

```
service nym-gateway start
```

This will cause your node to start at system boot time. If you restart your machine, the node will come back up automatically.

You can also do `service nym-gateway stop` or `service nym-gateway restart`.

Note: if you make any changes to your systemd script after you've enabled it, you will need to run:

```
systemctl daemon-reload
```

This lets your operating system know it's ok to reload the service configuration.

## Gateway related Validator API endpoints
Numerous gateway related API endpoints are documented on the Validator API's [Swagger Documentation](https://validator.nymtech.net/api/swagger/index.html). There you can also try out various requests from your broswer, and download the response from the API. Swagger will also show you what commands it is running, so that you can run these from an app or from your CLI if you prefer.
