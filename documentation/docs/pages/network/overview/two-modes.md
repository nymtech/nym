# Two Modes: dVPN and Mixnet

NymVPN provides two distinct privacy modes. Both operate on the same Nym Network infrastructure, but they make different tradeoffs between speed and protection level.

## dVPN mode

dVPN mode routes traffic through 2 hops—an Entry Gateway and an Exit Gateway. Traffic flows from your device to the Entry Gateway, then to the Exit Gateway, then to the destination.

```
User ──▶ Entry Gateway ──▶ Exit Gateway ──▶ Internet
```

This mode uses [Amnezia Wireguard](https://docs.amnezia.org/documentation/amnezia-wg/) to create a tunnel between you and the Entry Gateway, which then creates another tunnel to the Exit Gateway.

dVPN mode hides your IP from destination servers and splits knowledge between two independent operators—the Entry Gateway knows your IP but not your destination, while the Exit Gateway knows your destination but not your IP. However, it does not add timing delays or cover traffic. A sophisticated adversary observing both gateways could potentially correlate entry and exit timing.

Use dVPN mode for general web browsing, streaming video, downloads, and situations where speed matters and your adversaries are typical—ISPs, websites, advertisers.

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

Latency is higher, typically 200-500ms additional, due to the mixing delays, but this mode defeats timing correlation attacks and provides unobservability against even global passive adversaries.

Use Mixnet mode for whenever maximum privacy justifies the latency cost.

## SDK access

Developers using the [Nym SDKs](/developers) have access to Mixnet mode only, since dVPN mode is specific to the NymVPN application. Traffic moves slightly differently from SDK-based apps with slightly higher security properties, depending on which SDK modules a developer wishes to use.

```
User ──▶ Entry ──▶ Mix L1 ──▶ Mix L2 ──▶ Mix L3 ──▶ Exit ──▶ Internet
                     │           │           │
                  delay       delay       delay
                    +           +           +
                  mixing      mixing      mixing
```

Or

```
User ──▶ Entry ──▶ Mix L1 ──▶ Mix L2 ──▶ Mix L3 ──▶ Exit ──▶ Nym Client
                     │           │           │
                  delay       delay       delay
                    +           +           +
                  mixing      mixing      mixing
```

The top model - also used by NymVPN - essentially uses the Mixnet as a proxy service, similar to Tor, whereas the bottom one uses the Mixnet - and as such sends Sphinx packets - end-to-end. You can read more about this in TODO LINK TO DEV DOCS
