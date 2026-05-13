---
title: "Nym Network Architecture: How the Mixnet Works"
description: "Deep dive into Nym network architecture, cryptographic systems, and how the mixnet provides network-level privacy against end-to-end attackers."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-02-11"
---

# The Nym Network

The Nym Network is decentralised privacy infrastructure that protects against **network-level** surveillance. Unlike tools that focus on encrypting message content, Nym protects the metadata surrounding communication: who talks to whom, when, how often, and how much. This metadata is sufficient for observers to map relationships and build behavioural profiles even without access to any message content. See [The Privacy Problem](/network/overview/privacy-problem) for a fuller treatment.

Nym offers two operating modes with different privacy/performance trade-offs, both available through [NymVPN](https://nymvpn.com). Developers can also integrate Mixnet mode directly via the [Nym SDKs](/developers). See [Choosing a Mode](/network/overview/choosing-a-mode) for guidance on which fits a given threat model.

### NymVPN

[NymVPN](https://nymvpn.com) is a subscription-based application that provides access to both modes:
- **dVPN mode** routes traffic through 2 hops using WireGuard with enhanced layer encryption. Fast enough for browsing and streaming, with strong privacy against typical adversaries.
- **Mixnet mode** routes traffic through 5 hops with packet mixing, timing delays, and cover traffic. Every packet is the same size, each hop only sees the next destination, and a constant stream of dummy packets hides when real communication is occurring. Designed for privacy against adversaries capable of observing the entire network.

Both modes use the same underlying infrastructure.

### Developer SDKs

The [Nym SDKs](/developers) allow developers to embed mixnet functionality directly into applications, with the same privacy guarantees as NymVPN's Mixnet mode. SDK usage is currently free for development and testing. The SDKs do **not** provide access to dVPN mode.

## Paying for privacy without losing it

A fundamental weakness of traditional VPNs is that payment records can deanonymise users, since most providers link sessions to account IDs. Nym addresses this with **zk-nyms**: zero-knowledge anonymous credentials that prove payment without revealing any other information. Each credential covers a small chunk of bandwidth and is unlinkable to any other.

When you pay for NymVPN, your payment is converted into a credential that can be split and re-randomized. Each Gateway connection uses a fresh, unlinkable proof; the Gateway verifies that you have paid without learning who you are. Your subscription cannot be linked to your network activity, even by infrastructure operators.

## Further reading

- **Network architecture:** [Overview](/network/overview) · [dVPN Mode](/network/dvpn-mode) · [Mixnet Mode](/network/mixnet-mode) · [Cryptography](/network/cryptography) · [Infrastructure](/network/infrastructure) · [Reference](/network/reference)
- **Application development:** [Developer documentation](/developers)
- **Node operation:** [Operator documentation](/operators/introduction)
