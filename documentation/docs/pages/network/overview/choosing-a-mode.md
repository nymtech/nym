---
title: "Choosing Between dVPN and Mixnet Mode"
description: "When to use NymVPN's dVPN mode for low-latency browsing versus Mixnet mode for metadata protection against sophisticated adversaries."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Choosing a Mode

Both dVPN and Mixnet mode run on the same Nym infrastructure but protect against different things — dVPN keeps your IP hidden from destinations and splits trust across two operators, while Mixnet mode goes further by trying to make your traffic patterns invisible even to someone watching the entire network.

Architecturally, the two modes are quite different. **dVPN mode** routes traffic through 2 hops — an Entry Gateway and an Exit Gateway — connected via [AmneziaWG](https://docs.amnezia.org/documentation/amnezia-wg/), a WireGuard fork that adds traffic obfuscation to help evade protocol-level detection. This keeps latency low but offers no protection against timing analysis. **Mixnet mode** routes traffic through 5 hops — an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway — with each Mix Node adding a random delay and mixing your packets with those of other users. Combined with a constant stream of cover traffic, this makes timing correlation impractical even for adversaries capable of observing the entire network.

## Quick comparison

| | dVPN Mode | Mixnet Mode |
|---|---|---|
| **Hops** | 2 (Entry + Exit Gateway) | 5 (Entry + 3 Mix Nodes + Exit) |
| **Additional latency** | 50–150ms | 200–500ms |
| **Timing protection** | No | Yes (random delays per hop) |
| **Cover traffic** | No | Yes (constant dummy packets) |
| **Protects against** | ISPs, websites, advertisers, passive observers | Global passive adversaries, timing correlation, traffic analysis |
| **Access** | [NymVPN](https://nymvpn.com) | NymVPN and [Nym SDKs](/developers) |

## Use dVPN mode when

- You need low latency for browsing, streaming, or downloads
- Your adversaries are typical: ISPs monitoring traffic, websites tracking location, advertisers building profiles
- Speed matters more than protection against sophisticated traffic analysis
- You want the decentralization and payment privacy benefits of Nym without the latency cost of mixing

## Use Mixnet mode when

- Metadata protection is critical: journalism, activism, whistleblowing, legal consultations
- You face sophisticated adversaries who might monitor network traffic across multiple points
- You are willing to accept higher latency (200–500ms) for stronger privacy guarantees
- You need unlinkability and unobservability, not just IP hiding

## For developers

Developers using the [Nym SDKs](/developers) have access to **Mixnet mode only**—dVPN mode is specific to the NymVPN application.

There are two integration models available via the SDKs:

**As a proxy** (traffic exits to the internet):
```
Your App --> Entry --> Mix Nodes --> Exit --> Internet
```

**End-to-end** (traffic stays within the Mixnet):
```
Your App --> Entry --> Mix Nodes --> Exit --> Nym Client
```

The proxy model uses the Mixnet similarly to Tor's exit relay model, whereas the end-to-end model sends Sphinx packets the entire way. See the [integration overview](/developers/integrations) for more detail on choosing between these approaches.
