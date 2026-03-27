---
title: "Mixnet Mode"
description: "How Nym's Mixnet mode works: 5-hop routing through Mix Nodes with random delays, packet reordering, and cover traffic for unlinkability and unobservability."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Mixnet Mode

Mixnet mode routes traffic through 5 hops — an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway — with random delays, packet reordering, and cover traffic at each mixing layer. It is available through [NymVPN](https://nymvpn.com) and the [Nym SDKs](/developers).

## How it works

```
User --> Entry --> Mix L1 --> Mix L2 --> Mix L3 --> Exit --> Internet
                    |           |           |
                  delay       delay       delay
```

As a packet travels through the network, each Mix Node strips one layer of Sphinx encryption to reveal the address of the next hop, applies a random delay, and forwards the packet onward. No node ever sees both the origin and the final destination. Alongside real traffic, the client continuously generates [cover traffic](/network/mixnet-mode/cover-traffic) — dummy packets that are cryptographically indistinguishable from real ones — so that the stream of packets entering and leaving the network looks the same whether or not any real communication is occurring.

## Privacy properties

- **Unlinkability**: the random delays and packet reordering at each Mix Node destroy the timing signal that would otherwise allow an observer to correlate incoming packets with outgoing ones, or to connect successive packets from the same user. See [Packet Mixing](/network/mixnet-mode/mixing) for how this works in practice.
- **Unobservability**: because the client sends a constant stream of cover traffic regardless of whether real communication is occurring, an observer cannot determine when a user is active, how much of the traffic is genuine, or even whether a given connection is carrying any real data at all. See [Cover Traffic](/network/mixnet-mode/cover-traffic).
- **Resistance to traffic analysis**: uniform Sphinx packet sizes prevent content-type fingerprinting, and per-packet routing eliminates the long-lived circuits that make Tor susceptible to end-to-end correlation. See [Traffic Flow](/network/mixnet-mode/traffic-flow).

## Performance

Latency is higher than dVPN mode, typically 200-500ms additional, due to the mixing delays at each of the three Mix Node layers. This is the cost of timing obfuscation. For most messaging applications, this latency is acceptable. For real-time applications like video calls, dVPN mode may be more appropriate.

For help deciding between dVPN and Mixnet mode, see [Choosing a Mode](/network/overview/choosing-a-mode).

## Further reading

The following pages cover mixnet internals in detail:

- [Loopix Design](/network/mixnet-mode/loopix) explains the academic foundation
- [Traffic Flow](/network/mixnet-mode/traffic-flow) shows the packet journey with diagrams
- [Cover Traffic](/network/mixnet-mode/cover-traffic) explains how dummy packets provide unobservability
- [Packet Mixing](/network/mixnet-mode/mixing) covers timing delays and their importance
- [Anonymous Replies](/network/mixnet-mode/anonymous-replies) describes SURBs for bidirectional communication
