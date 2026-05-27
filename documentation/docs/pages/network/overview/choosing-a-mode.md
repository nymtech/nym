---
title: "Choosing Between dVPN and Mixnet Mode"
description: "When to use NymVPN's dVPN mode for low-latency browsing versus Mixnet mode for metadata protection against traffic analysis."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Choosing a Mode

Both modes run on the same Nym infrastructure but defend against different threat models. dVPN mode hides your IP and splits trust across two operators, and Mixnet mode additionally protects traffic patterns against adversaries capable of observing the entire network.

**dVPN mode** routes through 2 hops (Entry Gateway + Exit Gateway) connected via [AmneziaWG](https://docs.amnezia.org/documentation/amnezia-wg/), a WireGuard fork with traffic obfuscation to evade protocol-level detection. Latency is low, but there is no protection against timing analysis.

**Mixnet mode** routes through 5 hops (Entry Gateway, three Mix Node layers, Exit Gateway). Each Mix Node adds a random delay and mixes packets with those of other users. Combined with continuous cover traffic, this makes timing correlation impractical even for an adversary watching the entire network.

## Quick comparison

| | dVPN Mode | Mixnet Mode |
|---|---|---|
| **Hops** | 2 (Entry + Exit Gateway) | 5 (Entry + 3 Mix Nodes + Exit) |
| **Timing protection** | No | Yes (random delays per hop) |
| **Cover traffic** | No | Yes (constant dummy packets) |
| **Protects against** | ISPs, websites, advertisers, passive observers | Global passive adversaries, timing correlation, traffic analysis |
| **Access** | [NymVPN](https://nymvpn.com) | NymVPN and [Nym SDKs](/developers) |

## Use dVPN mode when

- Latency matters: browsing, streaming, downloads, video calls
- Your concern is ISPs, advertisers, and websites tracking you, not nation-state surveillance
- You want decentralised trust and payment privacy without the overhead of mixing

## Use Mixnet mode when

- Metadata exposure is dangerous: journalism, activism, whistleblowing, legal work
- Your adversary might be watching traffic across multiple network points
- Added latency is an acceptable trade for unlinkability and unobservability

## For developers

The [Nym SDKs](/developers) only expose **Mixnet mode**. dVPN mode is specific to the NymVPN application.

There are two integration models:

**Proxy** (traffic exits to the internet, analogous to Tor's exit relay model):
```
Your App --> Entry --> Mix Nodes --> Exit --> Internet
```

**End-to-end** (Sphinx-encrypted the entire way, traffic stays within the Mixnet):
```
Your App --> Entry --> Mix Nodes --> Exit --> Nym Client
```

See the [integration overview](/developers/integrations) for guidance on choosing between them.
