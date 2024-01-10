# NymVPN alpha

<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://player.vimeo.com/video/897010658?h=1f55870fe6&amp;badge=0&amp;autopause=0&amp;player_id=0&amp;app_id=58479" frameborder="0" allow="autoplay; fullscreen; picture-in-picture" style="position:absolute;top:0;left:0;width:100%;height:100%;" title="NYMVPN alpha demo 37C3"></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

We are honored to present NymVPN, a client that uses [Nym Mixnet](https://nymtech.net) to anonymise users entire internet traffic through either a 5-hop mixnet for a full network privacy or through the faster 2-hop decentralised VPN with extra features. At this event we have the unique opportunity to be part of the initial alpha testing. The following pages provides a how to guide, explaining steps to install and run NymVPN CLI and GUI on our testing environment Nym Sandbox. 
 
## NymVPN Guide & FAQ pages 

* [GNU/Linux](nym-vpn-linux.md)
* [Mac OS](nym-vpn-mac.md)
* [Testing scripts](nym-vpn-testing.md)
* [Troubleshooting](nym-vpn-troubleshooting.md)
* [NymVPN FAQ](nym-vpn-faq.md)

## NymVPN

The following overview provides an easy introduction to the NymVPN alpha client. We recommend interested developers to begin with [Nym network overview](https://nymtech.net/docs/architecture/network-overview.html) and the [Mixnet traffic flow](https://nymtech.net/docs/architecture/traffic-flow.html) pages.

The default is to run in 5-hop mode: 

```
                      ┌─►mix──┐  mix     mix
                      │       │
            Entry     │       │                   Exit
client ───► Gateway ──┘  mix  │  mix  ┌─►mix ───► Gateway ───► internet
                              │       │
                              │       │
                         mix  └─►mix──┘  mix
```

Users can switch to 2-hop only mode, which is a faster but less private option. In this mode traffic is only sent between the two Gateway's, and is not passed between Mix Nodes. 

The client can optionally do the first connection to the entry gateway using Wireguard. NymVPN uses Mullvad libraries for wrapping `wireguard-go` and to setup local routing rules to route all traffic to the TUN virtual network device.

## Goals of testing

This alpha testing will help:

* Stabilise NymVPN client
* Understand NymVPN client behavior in various setups (OS, connectivity, etc.)
* Stabilize the VPN infrastructure and improve its reliability / speed / features (e.g. IPv6 support)
* Load test the network in Sandbox environment and identify / anticipate potential weaknesses
 
 
```admonish info
Our alpha testing round is done live with some participants at the event. This guide will not work for everyone, as the NymVPN binaries aren't publicly accessible yet. Note that this setup of Nym testnet Sandbox environment is limited event and some of the configurations will not work in the future.

**If you commit to test NymVPN alpha, please start with the [user research form](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2) where all the steps will be provided**. 
```

