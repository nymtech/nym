---
title: "Nym Network Architecture: How the Mixnet Works"
description: "Deep dive into Nym network architecture, cryptographic systems, and how the mixnet provides network-level privacy against end-to-end attackers."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-02-11"
---

# The Nym Network

The Nym Network is decentralized privacy infrastructure that protects against **network-level** surveillance. It does this by protecting message *metadata*—who is communicating with whom, when, how often, and how much—from being able to be captured.

## The problem with metadata

When you send data across the internet, observers can see that communication has occurred in the form of the source and destination IP addresses of internet packets, the timing and frequency of transmissions, packet sizes, and other bits of information that over time can be used to build up inferences about the type of [device/browser you're using](https://browserleaks.com/ip), [your connection](https://browserleaks.com/tcp), and ultimately who you are. These observers include your ISP, internet infrastructure providers, governments, and large corporations.

Even when sending encrypted content (e.g. using messaging apps like Signal or SimpleX, or encrypted email providers), metadata can identify users by allowing observers to build up inferences and build behavioral profiles. Advances in machine learning in recent years has made these attacks increasingly practical, and spawned an entire industry dedicated to the capture and analysis of internet traffic.

## How Nym solves this

Every person and usecase has a different threat model - journalists in highly adversarial environments might be happy to accept higher latency and lower throughput when their safety is on the line, whereas your average user might just want to be 'private enough' to not be leaking everything they do to an ISP, passive government surveillance, or a centralised VPN provider.

As such, there are two 'modes' for sending traffic through Nym, each serving different needs. There are also two different ways to access the network:

### NymVPN

[NymVPN](https://nymvpn.com) is a subscription-based application that provides access to both modes:
- **dVPN mode** routes traffic through 2 hops using WireGuard with enhanced layer encryption—fast enough for browsing and streaming while still providing strong privacy against typical adversaries.
- **Mixnet mode** routes traffic through 5 hops with packet mixing, timing delays, and cover traffic, providing maximum privacy against sophisticated adversaries capable of observing the entire network. In the Mixnet, every packet is the same size, each hop only sees the next destination, packets are delayed and reordered to destroy timing patterns, and a constant stream of 'dummy' packets hides when real communication is occurring.

Both modes use the same underlying infrastructure.

### Developer SDKs

Developers can integrate mixnet functionality directly into applications using the [Nym SDKs](/developers). This provides the same privacy guarantees as NymVPN's mixnet mode and is currently free for development and testing. The SDKs do **not** provide access to dVPN mode, which is currently specific to the NymVPN application.

## Paying for privacy without losing it

A fundamental problem with VPNs and privacy services is that payment information can easily deanonymize users (e.g. most VPNs will link a user's session to their account ID). Nym solves this with **zk-nyms**—zero-knowledge anonymous credentials that allow you to prove you've paid for a subscription without revealing **anything else** about you. Each are used for small chunks of bandwidth, and are unlinkable to each other.

When you pay for NymVPN access, your payment is converted to a cryptographic credential that can be split and re-randomized. Each time you connect to a new Gateway node (for example, you switch which server you want your connection to be partially routed through), you present a fresh, unlinkable proof. Gateways verify payment validity without learning your identity, and **your subscription cannot be linked to your network activity, even by infrastructure operators**.

## Documentation structure

This documentation covers the network architecture and protocols:
- [Overview](/network/overview): high-level concepts.
- [dVPN Mode](/network/dvpn-mode): more detail about the protocol and traffic flow of dVPN mode.
- [Mixnet Mode](/network/mixnet-mode): more detail about the protocol and traffic flow of Mixnet mode.
- [Cryptography](/network/cryptography): covers the underlying primitives (including zk-nyms).
- [Infrastructure](/network/infrastructure): blockchain and node architecture.
- [Reference](/network/reference): technical specifications.

For building applications and intergrating existing apps with the Mixnet, see the [Developer Documentation](/developers).

If you wish to take part in the network as a Node Operator, see the [Operator Documentation](/operators/introduction).
