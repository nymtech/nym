# Nym Node

```admonish note
If you are a `nym-mixnode` or `nym-gateway` operator and you are not familiar with the binary changes called *Project Smoosh*, you can read the archived [Smoosh FAQ](../archive/smoosh-faq.md) page.
```

NYM NODE is a tool for running a node within the Nym network. Nym Nodes containing functionality such as `mixnode`, `entry-gateway` and `exit-gateway` are fundamental components of Nym Mixnet architecture. Nym Nodes are ran by decentralised node operators.

To setup any type of Nym Node, start with either building [Nym's platform](../binaries/building-nym.md) from source or download [pre-compiled binaries](../binaries/pre-built-binaries.md) on the [configured server (VPS)](vps-setup.md) where you want to run the node. Nym Node will need to be bond to [Nym's wallet](wallet-preparation.md). Follow [preliminary steps](preliminary-steps.md) page before you initialise and run a node.

```admonish info
**Migrating an existing node to a new `nym-node` is simple. The steps are documented on the [next page](setup.md#migrate)**
```

## Steps for Nym Node Operators

Once VPS and Nym wallet are configured, binaries ready, the operators of `nym-node` need to:

#### 1. [Setup & Run](setup.md) the node
#### 2. [Configure](configuration.md) the node (and optionaly WSS, reversed proxy, automation)
#### 3. [Bond](bonding.md) the node to the Nym API, using Nym wallet

## Quick `nym-node --mode exit-gateway` Setup

A fast ten commands deployment to get and setup your node, configure networking and connectivity and check that it all works fine by getting two free jokes through the mixnet.

```admonish caution
If you are not well familiar with `nym-node` setup, automation, and `nymtun0` configuration, follow the [steps above](#steps-for-nym-node-operators) page by page. You can use this flow as a reference later on.
```

1. [Get](../binaries/pre-built-binaries.md) or [build](../binaries/building-nym.md) the latest `nym-node` binary

2. Get [network_tunnel_manager.sh](https://gist.github.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77) script and grant permissions
```sh
curl -o network_tunnel_manager.sh -L https://gist.githubusercontent.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77/raw/9d785d6ee3aa2970553633eccbd89a827f49fab5/network_tunnel_manager.sh && chmod +x network_tunnel_manager.sh
```

3. If you have a running `nym-node` service, stop it now `service nym-node stop`

4. Check Nymtun IP tables:
```sh
sudo ./network_tunnel_manager.sh check_nymtun_iptables
```
 - if there's no process running it shouldn't get anything

5. Display IPv6: 
```sh
sudo ./network_tunnel_manager.sh fetch_and_display_ipv6
```
 - if you have a `global ipv6` address this is good
~~~admonish example collapsible=true title="Correct `./network_tunnel_manager.sh fetch_and_display_ipv6` output:"
```sh
iptables-persistent is already installed.
Using IPv6 address: 2001:db8:a160::1/112 #the address will be different for you
operation fetch_ipv6_address_nym_tun completed successfully.
```
~~~

6. Apply the rules: 
```sh
sudo ./network_tunnel_manager.sh apply_iptables_rules
``` 
  - and check them again like in point 3.

7. (If you didn't have a `nym-node` service yet) Create `systemd` [automation and configuration file](configuration.md#systemd), reload, enable

8. Start `nym-node` service:
```sh
sudo service start nym-node && journalctl -u nym-node -f -n 100
```

9. After a minute of running properly, check `nymtun0`:
```sh
ip addr show nymtun0
```

~~~admonish example collapsible=true title="Correct `ip addr show nymtun0` output:"
```sh
# your addresses will be different
8: nymtun0: <POINTOPOINT,MULTICAST,NOARP,UP,LOWER_UP> mtu 1420 qdisc fq_codel state UNKNOWN group default qlen 500
    link/none
    inet 10.0.0.1/16 scope global nymtun0
       valid_lft forever preferred_lft forever
    inet6 2001:db8:a160::1/112 scope global
       valid_lft forever preferred_lft forever
    inet6 fe80::ad08:d167:5700:8c7c/64 scope link stable-privacy
       valid_lft forever preferred_lft forever`
```
~~~

10. Validate your IPv6 and IPv4 networking by running a joke via Mixnet: 
```sh
sudo ./network_tunnel_manager.sh joke_through_the_mixnet
```
