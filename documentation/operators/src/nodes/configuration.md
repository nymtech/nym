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


## Connectivity Test and Configuration

```admonish info
**This chapter is relevant only for operators running `entry-gateway` and `exit-gateway` mode.**
```

With embedded Network Requester and IP Packet Router (modules routing data for the Mixnet and NymVPN traffic), there are more connectivity requirements on `nym-node` VPS setup. While we're working on Rust implementation to have these settings as a part of the binary build, in the meantime we wrote two scripts [`nym_network_diagnostics.sh`](https://gist.github.com/tommyv1987/a5fb30f5966e9d7bfbce58d88a85c0c1) and [`enable_networking_for_nym_nodes.sh`](https://gist.github.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77) to support the operators to configure their servers.

1. Download `nym_network_diagnostics.sh`, make executable and run:

```sh
curl -s -L -o gateway_network_check.sh https://gist.githubusercontent.com/tommyv1987/a5fb30f5966e9d7bfbce58d88a85c0c1/raw/27acbdbeecf9e04a0faee6a96e717aa7231935ef/nym_network_diagnostics.sh && chmod u+x gateway_network_check.sh  && ./gateway_network_check.sh
```
~~~admonish example collapsible=true title="An overview of `gateway_network_check.sh` flow"
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

  - To run the test next time, just enter `./gateway-network-check.sh`

2. Check out the outcome. The important parts are:
  - Status `1` on the `IPv4` and `IPv6 forwarding status`
  - `IPv4` and `IPv6 firewall rules` showing `state RELATED, ESTABLISHED`
  - Running `ping` on `IPv4` and `IPv6`
  - `checking internet and mixnet connectivity (IPv4) via nymtun0...` working
  - `checking internet and mixnet connectivity (IPv6) via nymtun0...` working
  - Below is an example correct outcome of the test
~~~admonish example collapsible=true title="A correct output of `enable_network_diagnostics.sh` flow"
```sh
---------------------------------------

checking IPv4 forwarding status...
1
---------------------------------------

checking IPv6 forwarding status...
1
---------------------------------------

checking UFW firewall Status...
Status: active
Logging: on (low)
Default: deny (incoming), allow (outgoing), deny (routed)
New profiles: skip

To                         Action      From
--                         ------      ----
22,1789,1790,8000,9000,9001/tcp ALLOW IN    Anywhere
9001/tcp                   ALLOW IN    Anywhere
8080                       ALLOW IN    Anywhere
443                        ALLOW IN    Anywhere
22,1789,1790,8000,9000,9001/tcp (v6) ALLOW IN    Anywhere (v6)
9001/tcp (v6)              ALLOW IN    Anywhere (v6)
8080 (v6)                  ALLOW IN    Anywhere (v6)
443 (v6)                   ALLOW IN    Anywhere (v6)

---------------------------------------

network Device: eth0
---------------------------------------

inspecting IPv4 firewall rules...
Chain FORWARD (policy DROP 0 packets, 0 bytes)
31880 2272K ufw-reject-forward  all  --  *      *       0.0.0.0/0            0.0.0.0/0
31880 2272K ACCEPT     all  --  nymtun0 eth0    0.0.0.0/0            0.0.0.0/0
    0     0 ACCEPT     all  --  eth0   nymtun0  0.0.0.0/0            0.0.0.0/0            state RELATED,ESTABLISHED
---------------------------------------


inspecting IPv6 firewall rules...
Chain FORWARD (policy DROP 0 packets, 0 bytes)
 2162  636K ufw6-reject-forward  all      *      *       ::/0                 ::/0
 2162  636K ACCEPT     all      nymtun0 eth0    ::/0                 ::/0
    0     0 ACCEPT     all      eth0   nymtun0  ::/0                 ::/0                 state RELATED,ESTABLISHED
---------------------------------------

examining IPv4 routing table...
default via 169.254.0.1 dev eth0 proto static onlink
10.0.0.0/16 dev nymtun0 proto kernel scope link src 10.0.0.1
---------------------------------------

examining IPv6 routing table...
::1 dev lo proto kernel metric 256 pref medium
2001:db8:a160::/112 dev nymtun0 proto kernel metric 256 pref medium
2a02:4780:12:3853::/64 dev eth0 proto kernel metric 256 pref medium
fe80::/64 dev eth0 proto kernel metric 256 pref medium
fe80::/64 dev nymtun0 proto kernel metric 256 pref medium
default via fe80::1 dev eth0 proto static metric 1024 onlink pref medium
---------------------------------------

checking IPv4 connectivity (example: google.com)...
PING google.com(bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e)) 56 data bytes
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=1 ttl=57 time=2.92 ms
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=2 ttl=57 time=2.81 ms
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=3 ttl=57 time=2.72 ms
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=4 ttl=57 time=2.82 ms

--- google.com ping statistics ---
4 packets transmitted, 4 received, 0% packet loss, time 3005ms
rtt min/avg/max/mdev = 2.718/2.817/2.924/0.072 ms
---------------------------------------

checking IPv6 connectivity (example: google.com)...
PING google.com(bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e)) 56 data bytes
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=1 ttl=57 time=2.69 ms
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=2 ttl=57 time=2.77 ms
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=3 ttl=57 time=2.71 ms
64 bytes from bom05s12-in-x0e.1e100.net (2404:6800:4009:80a::200e): icmp_seq=4 ttl=57 time=2.71 ms

--- google.com ping statistics ---
4 packets transmitted, 4 received, 0% packet loss, time 3004ms
rtt min/avg/max/mdev = 2.691/2.720/2.769/0.029 ms
---------------------------------------

checking internet and mixnet connectivity (IPv4) via nymtun0...
if a joke is returned there's connectivity through ipv4 and the nymtun, are you ready?
"Geology rocks, but Geography is where it's at!"
---------------------------------------

checking Internet and mixnet connectivity (IPv6) via nymtun0...
if a joke is returned, there's connectivity through IPv6 and the nymtun. are you ready?
joke fetched successfully:
"A man tried to sell me a coffin today. I told him that's the last thing I need."
machine check complete

```
~~~

3. Download `enable_network_diagnostics.sh`, make executable and run:

```sh
curl -s -L -o enable_networking_for_nym_nodes.sh https://gist.githubusercontent.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77/raw/7adf0d06d83561598c908e29b4a715c11f6432bf/enable_networking_for_nym_nodes.sh && chmod u+x enable_networking_for_nym_nodes.sh && sudo ./enable_networking_for_nym_nodes.sh
```
~~~admonish example collapsible=true title="An overview of `enable_network_diagnostics.sh` flow"
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

  - You can also make a sanity check, by running this test as separated steps:
```sh
sudo ./enable_networking_for_nym_nodes.sh check_nymtun_iptables

sudo ./enable_networking_for_nym_nodes.sh apply_all_iptable_rules_nymtun
```

4. After running `enable_network_diagnostics.sh`, re-run `./gateway-network-check.sh` and check the outcome. If there are still problems, please refer to [troubleshooting section](../troubleshooting/vps-setup.md#incorrect-gateway-network-check)

If all the setup went smooth, your server is ready to connect `nym-node` with the rest of the Mixnet. There are a few more good suggestions for `nym-node` VPS configuration, especially to be considered for Gateway functionality, like Web Secure Socket or Reversed Proxy setup. Visit [Proxy configuration](proxy-configuration) page to see the guides.
