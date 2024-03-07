# NymVPN alpha

<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://player.vimeo.com/video/897010658?h=1f55870fe6&amp;badge=0&amp;autopause=0&amp;player_id=0&amp;app_id=58479" frameborder="0" allow="autoplay; fullscreen; picture-in-picture" style="position:absolute;top:0;left:0;width:100%;height:100%;" title="NYMVPN alpha demo 37C3"></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

**Nym proudly presents NymVPN alpha** - a client that uses [Nym Mixnet](https://nymtech.net) to anonymise all of a user's internet traffic through either a 5-hop mixnet (for a full network privacy) or the faster 2-hop decentralised VPN (with some extra features).


**You are invited to take part in the alpha testing** of this new application. The following pages provide a how-to guide, explaining steps to install and run NymVPN [CLI](cli.md) and [GUI](gui.md) on the Sandbox testnet environment.

**Here is how**

1. Go to the NymVPN [testers form]({{nym_vpn_form_url}})
2. Please consent to the GDPR so we can use the results
3. To test the GUI, [go here](gui.md)
4. To test the CLI, [go here](cli.md)
5. Fill and submit the [form!]({{nym_vpn_form_url}})
6. Join the [NymVPN matrix channel](https://matrix.to/#/#NymVPN:nymtech.chat) if you have any questions, comments or blockers

***NymVPN alpha testing will last from 15th of January - 15th of February.***

*NOTE: NymVPN alpha is experimental software for [testing purposes](testing.md) only.*


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

Users can switch to 2-hop only mode, which is a faster but less private option. In this mode traffic is only sent between the two Gateways, and is not passed between Mix Nodes.

The client can optionally do the first hop (local client to Entry Gateway) using Wireguard. NymVPN uses Mullvad libraries for wrapping `wireguard-go` and to setup local routing rules to route all traffic to the TUN virtual network device.

## NymVPN Resources & Guides

* [NymVPN webpage](https://nymvpn.com)
* [Alpha release page]({{nym_vpn_latest_binary_url}})
* [NymVPN application (GUI) guide](gui.md)
* [NymVPN Command Line Interface (CLI) guide](cli.md)
* [Troubleshooting](troubleshooting.md)
* [NymVPN FAQ](faq.md)
* [NymVPN matrix channel](https://matrix.to/#/#NymVPN:nymtech.chat)
