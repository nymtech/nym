---
title: "Nym Network Architecture: How the Mixnet Works"
description: "Deep dive into Nym network architecture, cryptographic systems, and how the mixnet provides network-level privacy against end-to-end attackers."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-02-11"
---

# The Nym Network

The Nym Network is a decentralized privacy infrastructure that protects against network-level surveillance. While traditional encryption protects message *content*, the Nym Network protects *metadata*—who is communicating with whom, when, how often, and how much.

## The problem with metadata

When you send data across the internet, observers can see that communication occurred, the source and destination IP addresses, timing and frequency of transmissions, and packet sizes. These observers include your ISP, internet infrastructure providers, governments, and large corporations.

Even with encrypted content, this metadata can identify users, infer relationships, and build behavioral profiles. Machine learning makes these attacks increasingly practical, and what was once theoretical is now a documented reality.

## How Nym solves this

The Nym Network defeats metadata surveillance by making all traffic look identical. Every packet is the same size. Each hop only sees the next destination. Packets are delayed and reordered to destroy timing patterns. A constant stream of dummy packets hides when real communication is occurring.

The network consists of over 600 nodes operated independently across nearly 60 countries. There is no single point of trust or failure.

## Accessing the network

There are two ways to use the Nym Network, each serving different needs.

### NymVPN

[NymVPN](https://nymvpn.com) is a paid application that provides two privacy modes. **dVPN mode** routes traffic through 2 hops using WireGuard with enhanced layer encryption—fast enough for browsing and streaming while still providing strong privacy against typical adversaries. **Mixnet mode** routes traffic through 5 hops with packet mixing, timing delays, and cover traffic, providing maximum privacy against sophisticated adversaries capable of observing the entire network.

Both modes use the same underlying infrastructure. To external observers, traffic from either mode is indistinguishable.

### Developer SDKs

Developers can integrate mixnet functionality directly into applications using the [Nym SDKs](/developers). This provides the same privacy guarantees as NymVPN's mixnet mode and is currently free for development and testing. The SDKs do not provide access to dVPN mode, which is specific to the NymVPN application.

## Paying for privacy without losing it

A fundamental problem with VPNs and privacy services is that payment information can deanonymize users. The Nym Network solves this with **zk-nyms**—zero-knowledge anonymous credentials that prove you've paid without revealing who you are.

When you pay for NymVPN access, your payment is converted to a cryptographic credential that can be split and re-randomized. Each time you connect, you present a fresh, unlinkable proof. Gateways verify payment validity without learning your identity. Your subscription cannot be linked to your network activity, even by Nym infrastructure operators.

## Documentation structure

This documentation covers the network architecture and protocols. Start with the [Overview](/network/overview) for high-level concepts, then explore [dVPN Mode](/network/dvpn-mode) or [Mixnet Mode](/network/mixnet-mode) depending on your interest. The [Cryptography](/network/cryptography) section covers the underlying primitives including zk-nyms. [Infrastructure](/network/infrastructure) explains the blockchain and node architecture. [Reference](/network/reference) contains technical specifications.

For building applications on the mixnet, see the [Developer Documentation](/developers).
