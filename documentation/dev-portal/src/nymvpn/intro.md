# NymVPN alpha

<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://player.vimeo.com/video/897010658?h=1f55870fe6&amp;badge=0&amp;autopause=0&amp;player_id=0&amp;app_id=58479" frameborder="0" allow="autoplay; fullscreen; picture-in-picture" style="position:absolute;top:0;left:0;width:100%;height:100%;" title="NYMVPN alpha demo 37C3"></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

**NymVPN alpha** is a client that uses [Nym Mixnet](https://nymtech.net) to anonymise all of a user's internet traffic through either a 5-hop mixnet (for a full network privacy) or the faster 2-hop decentralised VPN (with some extra features).


**You are invited to take part in the alpha testing** of this new application. Register for private testing round at [nymvpn.com](https://nymvpn.com/en), that will grant you access to the [download page](https://nymvpn.com/download). Visit [NymVPN Support & FAQ](https://nymvpn.com/en/support) or join the [NymVPN matrix channel](https://matrix.to/#/#NymVPN:nymtech.chat) if you have any questions, comments or blockers.

Checkout the [release page](https://github.com/nymtech/nym-vpn-client/releases) for available binaries.

*NOTE: NymVPN alpha is experimental software for testing purposes only.*


## NymVPN Overview

To understand what's under the hood of NymVPN and the mixnet, we recommend interested developers to begin with [Nym network overview](https://nymtech.net/docs/architecture/network-overview.html) and the [Mixnet traffic flow](https://nymtech.net/docs/architecture/traffic-flow.html) pages.

The default setup of NymVPN is to run in 5-hop mode (mixnet):

```
                      ┌─►mix──┐  mix     mix
                      │       │
            Entry     │       │                   Exit
client ───► Gateway ──┘  mix  │  mix  ┌─►mix ───► Gateway ───► internet
                              │       │
                              │       │
                         mix  └─►mix──┘  mix
```

Users can switch to 2-hop only mode, which is a faster but less private option. In this mode traffic is only sent between the two Gateways, and is not passed between Mix Nodes. It uses Mixnet Sphinx packets with shorter, fixed routes, which improve latency, but doesn't offer the same level of protection as the 5 hop mode.
<!-- TO BE IMPLEMENTED:
Users can switch to 2-hop only mode, which is a faster but less private option. In this mode traffic is only sent between the two Gateways, and is not passed between Mix Nodes. The client than use two wireguard tunnels with the entry and exit gateway, the Exit Gateway one being tunnelled itself through the entry gateway tunnel. NymVPN uses Mullvad libraries for wrapping `wireguard-go` and to setup local routing rules to route all traffic to the TUN virtual network device.
-->
