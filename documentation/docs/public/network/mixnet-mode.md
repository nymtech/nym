---
title: Mixnet Mode
description: How Nym's Mixnet mode works: 5-hop routing through Mix Nodes with random delays, packet reordering, and cover traffic for unlinkability and unobservability.
url: https://nym.com/docs/network/mixnet-mode
---

# Mixnet Mode

export const NetworkDiagram = dynamic(
  () => import('../../components/threat-model/NetworkDiagram').then((m) => m.NetworkDiagram),
  { ssr: false },
)

export const mixScenario = requireGenericScenario('mixnet')

Mixnet mode is a [transport](/network/threat-model/two-layer-model) choice. It routes traffic through 5 hops: an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway. Each mixing layer adds random delays, reorders packets, and injects cover traffic. Available through [NymVPN](https://nymvpn.com) and the [Nym SDKs](/developers).

Its distinctive strength is against network observers. Mixing delays, Poisson sending, and cover traffic hide the traffic pattern in transit, which a plain [dVPN](/network/dvpn-mode) cannot do. Against the destination it is equivalent to dVPN: both hide the client IP, and neither closes the timing or content vectors that the destination sees on its own.

## How it works

```
User --> Entry --> Mix L1 --> Mix L2 --> Mix L3 --> Exit --> Internet
                    |           |           |
                  delay       delay       delay
```

Each Mix Node strips one layer of [Sphinx](/network/cryptography/sphinx) encryption to learn the next hop, holds the packet for a random delay, then forwards it. No node ever sees both the origin and the final destination. The client also continuously sends [cover traffic](/network/mixnet-mode/cover-traffic), dummy packets cryptographically indistinguishable from real ones, so an observer sees a constant stream of identical packets regardless of whether any real communication is taking place.

## What it protects against network observers

Against the [local](/network/threat-model/actors#actor-L3L) and [global](/network/threat-model/actors#actor-L3G) network observer, the mixnet is what separates it from a dVPN:

- **Unlinkability**: the random delays and reordering at each Mix Node destroy the timing signal an observer would need to correlate incoming and outgoing packets, or to connect successive packets from the same user. See [Packet Mixing](/network/mixnet-mode/mixing).
- **Unobservability**: because cover traffic is constant, an observer cannot determine when a user is active or what fraction of the traffic is real. See [Cover Traffic](/network/mixnet-mode/cover-traffic).
- **Resistance to traffic analysis**: uniform Sphinx packet sizes prevent content-type fingerprinting, and per-packet routing eliminates the long-lived circuits that make other anonymity networks susceptible to end-to-end correlation. See [Traffic Flow](/network/mixnet-mode/traffic-flow).

## What it does not close at the destination

Against the [destination](/network/threat-model/actors#actor-L2), mixnet mode behaves like a dVPN. It hides the client IP, so the destination sees the exit gateway's IP rather than yours. It does not, on its own, close the timing or content vectors the destination observes: the connection arriving from the exit gateway is an ordinary end-to-end connection, so every request and its full content arrive together. A fixed exit gateway also behaves like a single dVPN exit, linkable at the destination within a session.

Closing those vectors is a separate layer: [baseline hygiene](/developers/swizzle) disciplines the timing and shape of the requests themselves, independent of transport. See [the two-layer model](/network/threat-model/two-layer-model) for why transport and hygiene are separate problems.

## Performance

The three mixing layers add additional latency. This is acceptable for messaging, file transfers, and most API calls, but unsuitable for real-time applications like video calling. For those, [dVPN mode](/network/dvpn-mode) is more appropriate.

## Further reading

The following pages cover mixnet internals in detail:

- [Loopix Design](/network/mixnet-mode/loopix) explains the academic foundation of Nym's Mixnet design
- [Traffic Flow](/network/mixnet-mode/traffic-flow) shows the packet journey with diagrams
- [Cover Traffic](/network/mixnet-mode/cover-traffic) explains how dummy packets provide unobservability
- [Packet Mixing](/network/mixnet-mode/mixing) covers timing delays and their importance
- [Anonymous Replies](/network/mixnet-mode/anonymous-replies) describes SURBs for bidirectional communication
