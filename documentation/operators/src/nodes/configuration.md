# Nym Node Configuration

## Basic Changes

Nym Node can be configured directly by editing the config file (`config.toml`) located at `~/.nym/nym-nodes/<ID>/config/config.toml` (by default `~/.nym/nym-nodes/default-nym-node/config/config.toml`) or through commands on the binary.

### Commands & Examples

Disable sharing of system hardware info with the network:

```sh
./nym-node run --id <ID> --deny-init --mode entry-gateway -w --expose-system-hardware false --expose-system-info false
```

Note: `--expose-system-info false` supersedes `--expose-system-hardware false`. If both are present with conflicting values, the system hardware will not be shown.


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

To automate with `systemd` use this init service file by saving it as `/etc/systemd/system/nym-node.service` and follow the [next steps](#following-steps-for-nym-nodes-running-as-systemd-service).

1. Open text editor
```sh
nano /etc/systemd/system/nym-node.service
```

2. Paste this file
```ini
[Unit]
Description=Nym Node
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=<USER>
LimitNOFILE=65536
ExecStart=/home/<USER>/<PATH>/nym-node run # add all the flags you use to run your node
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
journalctl -u nym-node -f
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
