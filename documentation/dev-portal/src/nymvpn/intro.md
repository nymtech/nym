# NymVPN alpha

<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://player.vimeo.com/video/897010658?h=1f55870fe6&amp;badge=0&amp;autopause=0&amp;player_id=0&amp;app_id=58479" frameborder="0" allow="autoplay; fullscreen; picture-in-picture" style="position:absolute;top:0;left:0;width:100%;height:100%;" title="NYMVPN alpha demo 37C3"></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

**Nym proudly presents NymVPN Alpha** - a client that uses [Nym Mixnet](https://nymtech.net) to anonymise all of a user's internet traffic through either a 5-hop mixnet (for a full network privacy) or the faster 2-hop decentralised VPN (with some extra features). 


**You are invited to take part in the alpha testing** of this new application. The following pages provide a how-to guide, explaining steps to install and run NymVPN [CLI](cli.md) and [GUI](gui.md) on the Sandbox testnet environment as well as provide some scripts for [qualitative testing](testing.md).

**Here is how**

1. Go to the NymVPN [testers form](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2)
2. Please consent to the GDPR so we can use the results
3. To test the GUI, [go here](gui.md)
4. To test the CLI, [go here](cli.md)
5. Run [qualitative testing script](testing.md)
6. Fill and submit the [form!](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2)
7. Join the [NymVPN matrix channel](https://matrix.to/#/#NymVPN:nymtech.chat) if you have any questions, comments or blockers

***NymVPN Alpha testing will last from 15th of January - 15th of February.***

*NOTE: NymVPN Alpha is experimental software for testing purposes only.* 


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

## NymVPN Documanetation Pages

* [NymVPN Application (GUI)](gui.md)
* [NymVPN Command Line Interface (CLI)](cli.md)
* [Testing scripts](testing.md)
* [Troubleshooting](troubleshooting.md)
* [NymVPN FAQ](faq.md)

