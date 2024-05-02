# Maintenance

## Useful commands

> Adding `--no-banner` startup flag will prevent Nym banner being printed even if run in tty environment.

**build-info**

A `build-info` command prints the build information like commit hash, rust version, binary version just like what command `--version` does. However, you can also specify an `--output=json` flag that will format the whole output as a json, making it an order of magnitude easier to parse.

For example `./target/debug/nym-network-requester --no-banner build-info --output json` will return:

```sh
{"binary_name":"nym-network-requester","build_timestamp":"2023-07-24T15:38:37.00657Z","build_version":"1.1.23","commit_sha":"c70149400206dce24cf20babb1e64f22202672dd","commit_timestamp":"2023-07-24T14:45:45Z","commit_branch":"feature/simplify-cli-parsing","rustc_version":"1.71.0","rustc_channel":"stable","cargo_profile":"debug"}
```

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
# for nym-node
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

> Replace `<NODE>` variable with type of node you run, preferably `nym-node` (depreciated `nym-mixnode`, `nym-gateway` or `nym-network-requester`).

### Automating your node with nohup, tmux and systemd

Although it’s not totally necessary, it's useful to have the Mix Node automatically start at system boot time. We recommend to run your remote operation via [`tmux`](maintenance.md#tmux) for easier management and a handy return to your previous session. For full automation, including a failed node auto-restart and `ulimit` setup, [`systemd`](maintenance.md#systemd) is a good choice.

> Do any of these steps and run your automated node before you start bonding process!

#### nohup

`nohup` is a command with which your terminal is told to ignore the `HUP` or 'hangup' signal. This will stop the node process ending if you kill your session.

```sh
nohup ./<NODE> run <OTHER_FLAGS> # use all the flags you use to run your node
```

#### tmux

One way is to use `tmux` shell on top of your current VPS terminal. Tmux is a terminal multiplexer, it allows you to create several terminal windows and panes from a single terminal. Processes started in `tmux` keep running after closing the terminal as long as the given `tmux` window was not terminated.

Use the following command to get `tmux`.

| Platform | Install Command |
| :---      | :---             |
| Arch Linux|`pacman -S tmux`             |
| Debian or Ubuntu|`apt install tmux`      |
| Fedora|`dnf install tmux`                 |
| RHEL or CentOS|`yum install tmux`          |
|  macOS (using Homebrew | `brew install tmux`    |
| macOS (using MacPorts) | `port install tmux`    |
|               openSUSE | `zypper install tmux`  |

In case it didn't work for your distribution, see how to build `tmux` from [version control](https://github.com/tmux/tmux#from-version-control).

**Running tmux**

Now you have installed tmux on your VPS, let's run a Mix Node on tmux, which allows you to detach your terminal and let your `<NODE>` run on its own on the VPS.

* Pause your `<NODE>`
* Start tmux with the command
```sh
tmux
```
* The tmux terminal should open in the same working directory, just the layout changed into tmux default layout.
* Start the `<NODE>` again with a command:
```sh
./<NODE> run <OTHER_FLAGS> # use all the flags you use to run your node
```
* Now, without closing the tmux window, you can close the whole terminal and the `<NODE>` (and any other process running in tmux) will stay active.
* Next time just start your teminal, ssh into the VPS and run the following command to attach back to your previous session:
```sh
tmux attach-session
```
* To see keybinding options of tmux press `ctrl`+`b` and after 1 second `?`

#### systemd

##### For Nymvisor
> Since you're running your node via a Nymvisor instance, as well as creating a Nymvisor `.service` file, you will also want to **stop any previous node automation process you already have running**.

To automate with `systemd` use this init service file by saving it as `/etc/systemd/system/nymvisor.service` and follow the [next steps](#following-steps-for-nym-nodes-running-as-systemd-service).

1. Open text editor
```sh
nano /etc/systemd/system/nymvisor.service
```

2. Paste this file

```
[Unit]
Description=Nymvisor <VERSION>
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=<USER> # replace this with whatever user you wish
LimitNOFILE=65536
ExecStart=/home/<USER>/<PATH>/nymvisor run --id <ID>
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

3. Save the file

```admonish note
Make sure your `ExecStart <FULL_PATH>` and `run` command are correct!

Example: If you have built nym in the `$HOME` directory on your server, your username is `jetpanther`, and node `<ID>` is `puma`, then the `ExecStart` line (command) in the script located in `/etc/systemd/system/nym-mixnode.service` for Nym Mixnode might look like this:
`ExecStart=/home/jetpanther/nym/target/release/nym-node run --id puma`.

Basically, you want the full `/<PATH>/<TO>/nym-mixnode run --id <WHATEVER-YOUR-NODE-ID-IS>`. If you are unsure about your `/<PATH>/<TO>/<NODE>`, then `cd` to your directory where you run your `<NODE>` from and run `pwd` command which returns the full path for you.
```


#### Following steps for Nym nodes running as `systemd` service

Once your init file is save follow these steps:

1. Reload systemctl to pickup the new unit file
```sh
systemctl daemon-reload
```

2. Enable the newly created service:

```sh
systemctl enable nym-node.service
```

3. Start your `<NODE>` as a `systemd` service:

```sh
service nym-node start
```

This will cause your `<NODE>` to start at system boot time. If you restart your machine, your `<NODE>` will come back up automatically.

**Useful systemd commands**

- You can monitor system logs of your node by running:
```sh
journalctl -u <NODE> -f
```

- Or check a status by running:
```sh
systemctl status <NODE>.service
# for example systemctl status nym-node.service
```

- You can also do `service <NODE> stop` or `service <NODE> restart`.

**Note:** if you make any changes to your `systemd` script after you've enabled it, you will need to run:

```sh
systemctl daemon-reload
```

This lets your operating system know it's ok to reload the service configuration. Then restart your `<NODE>`.


## Moving a node

In case of a need to move a Nym Node from one machine to another and avoiding to lose the delegation, here are few steps how to do it.

Assuming both machines are remote VPS.

* Make sure your `~/.ssh/<YOUR_KEY>.pub` is in both of the servers `~/.ssh/authorized_keys` file
* Create a `nym-nodes` folder in the target VPS. SSH in from your terminal and run:

```sh
# in case none of the nym configs was created previously
mkdir ~/.nym

#in case no nym Nym Node was initialized previously
mkdir ~/.nym/nym-nodes
```
* Move the node data (keys) and config file to the new machine by opening your **local terminal** (as that one's ssh key is authorized in both of the VPS) and running:
```sh
scp -r -3 <SOURCE_USER_NAME>@<SOURCE_HOST_ADDRESS>:~/.nym/nym-nodes <TARGET_USER_NAME>@<TARGET_HOST_ADDRESS>:~/.nym/nym-nodes/
```

**On new/target machine**

* Edit `~/.nym/nym-nodes/<ID>/config/config.toml` config with the new listening address IP.
* Setup the [systemd](#systemd) automation, reload the daemon and run the service, or just simply run the node if you don't use automation
* Change the node smart contract info via the wallet interface. Otherwise the keys will point to the old IP address in the smart contract, and the node will not be able to be connected, and it will fail up-time checks.

## Rename node local ID

Local node ID (not the identity key) is a name chosen by operators which defines where the nodes configuration data will be stored, where the ID determines the path to `~/.nym/nym-nodes/<ID>/`. This ID is never shared on the network.

Since the migration to [`nym-node`](nym-node.md), the use of ID as a flag `--ID <ID>` is no longer required when starting a new node. Nodes without an ID specified will be asigned an ID `default-nym-node`. The management of nodes is simplified and it makes it especially easier for operators managing multiple nodes through ansible and other automation scripts, as everything is stored at `~/.nym/nym-nodes/default-nym-node`.

If you already operate a `nym-node` and wish to change the local ID to `default-nym-node` or anything else, follow the steps below to do so.

```admonish note
In the example we use `default-nym-node` as a target `<ID>`, if you prefer to use another name, edit the syntax in the commands accordingly.
```

1. Copy the configuration directory to the new one
```sh
cp -r  ~/.nym/nym-nodes/<SOURCE_ID> ~/.nym/nym-nodes/default-nym-node/
```

2. Rename all `<SOURCE_ID>` occurences in `config.toml` to `default-nym-node`

```sh
# check occurences of the <SOURCE_ID>
grep -r  "<SOURCE_ID" ~/.nym/nym-nodes/default-nym-node/*
```
```admonish bug title="Caution!"
If your node `<SOURCE_ID>` is too generic (like `gateway` etc) and it occurs elsewhere than just a custom value, **do not use `sed` command but rewrite the values manually using a text editor!**
```

```sh
# rename it by using sed command
sed -i -e "s/<SOURCE_ID>/default-nym-node/g" ~/.nym/nym-nodes/default-nym-node/config/config.toml

# or manually by opening config.toml and rewriting each occurence of <SOURCE_ID>
nano ~/.nym/nym-nodes/default-nym-node/config/config.toml
```

3. Validate by rechecking the config file content
```sh
# either re-run
grep -r  "<SOURCE_ID>" ~/.nym/nym-nodes/default-nym-node/*

# or by reading the config file
less ~/.nym/nym-nodes/default-nym-node/config/config.toml
```
- Pay extra attention to the `hostname` line. Iin case its value was somehow correlated with the `<SOURCE_ID>` string you may need to correct it back.

4. Reload your [systemd service daemon](#systemd) and restart the service or simply restart the node if you don't use automation

5. If you double-checked that everything works fine, you can consider removing your old config directory

## Ports
All `<NODE>`-specific port configuration can be found in `$HOME/.nym/<NODE>/<YOUR_ID>/config/config.toml`. If you do edit any port configs, remember to restart your client and node processes.

### Nym Node: Minode mode port reference
| Default port | Use                       |
| ------------ | ------------------------- |
| `1789`       | Listen for Mixnet traffic |
| `1790`       | Listen for VerLoc traffic |
| `8000`       | Metrics http API endpoint |


### Nym Node: Gateway modes port reference
| Default port | Use                       |
|--------------|---------------------------|
| `1789`       | Listen for Mixnet traffic |
| `9000`       | Listen for Client traffic |
| `9001`       | WSS                       |

### Validator port reference
All validator-specific port configuration can be found in `$HOME/.nymd/config/config.toml`. If you do edit any port configs, remember to restart your validator.

| Default port | Use                                  |
|--------------|--------------------------------------|
| 1317         | REST API server endpoint             |
| 26656        | Listen for incoming peer connections |
| 26660        | Listen for Prometheus connections    |
