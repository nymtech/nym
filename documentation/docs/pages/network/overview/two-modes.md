# Two Modes: dVPN and Mixnet

NymVPN has two modes, each using the same underlying network infrastructure but handling traffic very differently.

## dVPN mode

dVPN mode routes traffic through 2 hops—an Entry Gateway and an Exit Gateway. Traffic flows from your device to the Entry Gateway, then to the Exit Gateway, then to the destination.

```
User ──▶ Entry Gateway ──▶ Exit Gateway ──▶ Internet
```

This mode uses [AmneziaWG](https://docs.amnezia.org/documentation/amnezia-wg/), a WireGuard fork that adds traffic obfuscation to help evade some forms of protocol detection. It creates a tunnel between you and the Entry Gateway, which then creates another tunnel to the Exit Gateway.

dVPN mode hides your IP from destination servers and splits knowledge between two independent operators—the Entry Gateway knows your IP but not your destination, while the Exit Gateway knows your destination but not your IP. However, it does not add timing delays or cover traffic. A sophisticated adversary observing both gateways could potentially correlate entry and exit timing.

See [Choosing a Mode](/network/overview/choosing-a-mode) for when to use dVPN vs Mixnet.

## Mixnet mode

Mixnet mode routes traffic through 5 hops—an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway. Each Mix Node adds a random delay and mixes your traffic with other packets passing through.

```
User ──▶ Entry ──▶ Mix L1 ──▶ Mix L2 ──▶ Mix L3 ──▶ Exit ──▶ Internet
                     │           │           │
                  delay       delay       delay
                    +           +           +
                  mixing      mixing      mixing
```

Beyond the additional hops, Mixnet mode generates constant cover traffic—dummy packets indistinguishable from real ones. This hides not just who you're communicating with, but when you're communicating.

Latency is higher, typically 200-500ms additional, due to the mixing delays, but this is what makes timing correlation attacks impractical even for adversaries watching the entire network.

For practical guidance on when to use each mode — and how developers access the network via SDKs — see [Choosing a Mode](/network/overview/choosing-a-mode).
