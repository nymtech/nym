---
title: "Nym Network Architecture: How the Mixnet Works"
description: "Deep dive into Nym network architecture, cryptographic systems, and how the mixnet provides network-level privacy against end-to-end attackers."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-02-11"
---

# The Nym Network

The Nym Network is a decentralized privacy infrastructure designed to protect users against network-level surveillance. Unlike traditional encryption which protects message *content*, the Nym Network protects *metadata*: who is communicating with whom, when, how often, and how much.

## The Problem

When you send data across the internet, observers (ISPs, infrastructure providers, governments, corporations) can see:
- That communication occurred
- Source and destination IP addresses
- Timing and frequency of transmissions
- Packet sizes and patterns

Even with encrypted content, this metadata can be used to identify users, infer relationships, and build behavioral profiles. Machine learning makes these attacks increasingly practical.

## The Solution: A Noise-Generating Network

The Nym Network defeats metadata surveillance by making all traffic look identical. It does this through:

- **Packet uniformity**: All packets are the same size
- **Encryption layering**: Each hop only sees the next destination
- **Traffic mixing**: Packets are delayed and reordered
- **Cover traffic**: Dummy packets hide real communication patterns

The network consists of 600+ nodes operated independently across nearly 60 countries, creating a decentralized infrastructure with no single point of trust or failure.

## Two Ways to Access the Network

### NymVPN (Consumer Product)

[NymVPN](https://nymvpn.com) is a paid application providing two privacy modes:

| Mode | Hops | Speed | Privacy Level | Best For |
|------|------|-------|---------------|----------|
| **dVPN Mode** | 2 | Fast | Strong | Browsing, streaming, general use |
| **Mixnet Mode** | 5 | Moderate | Maximum | Sensitive communications, high-threat scenarios |

Both modes use the same underlying network infrastructure. To external observers, traffic from either mode is indistinguishable.

- **dVPN Mode**: Routes traffic through 2 hops using WireGuard with enhanced layer encryption. Provides strong privacy against most adversaries while maintaining speed suitable for everyday use.

- **Mixnet Mode**: Routes traffic through 5 hops with packet mixing, timing delays, and cover traffic. Provides maximum privacy against sophisticated adversaries including those capable of observing the entire network.

### Developer SDKs (Mixnet Access)

Developers can integrate mixnet functionality directly into applications using the [Nym SDKs](../developers). This provides:

- Direct access to mixnet privacy (equivalent to NymVPN's mixnet mode)
- Message-based communication through the network
- Currently free for development and testing

The SDKs do not provide access to dVPN mode, which is specific to the NymVPN application.

## Network Components

The Nym Network consists of several types of infrastructure:

- **Nym Nodes**: Servers that route and mix traffic. They operate in different modes:
  - *Entry Gateways*: First hop into the network, manage client connections
  - *Mix Nodes*: Middle layers that mix and delay packets
  - *Exit Gateways*: Final hop, communicate with external services

- **Nyx Blockchain**: A Cosmos SDK chain that manages network topology, staking, and the credential system

- **Nym API**: Services that monitor network health and issue privacy-preserving access credentials (zk-nyms)

## Paying for Privacy (Without Losing It)

A fundamental problem with VPNs and privacy services is that payment information can deanonymize users. The Nym Network solves this with **zk-nyms**: zero-knowledge anonymous credentials that prove you've paid without revealing who you are.

When you pay for NymVPN access:
1. Payment is converted to a cryptographic credential
2. The credential can be split and re-randomized
3. Each time you connect, you present a fresh, unlinkable proof
4. Gateways verify payment validity without learning your identity

This means your subscription cannot be linked to your network activity, even by Nym infrastructure operators.

## Documentation Structure

This documentation is organized as follows:

- **[Overview](./overview)**: High-level explanations of network concepts and design
- **[dVPN Mode](./dvpn-mode)**: How the 2-hop decentralized VPN works (NymVPN)
- **[Mixnet Mode](./mixnet-mode)**: How the 5-hop mixnet works
- **[Cryptography](./cryptography)**: Encryption standards, Sphinx packets, and zk-nyms
- **[Infrastructure](./infrastructure)**: Nyx blockchain and node architecture
- **[Reference](./reference)**: Technical specifications and protocol details

For building applications on the mixnet, see the [Developer Documentation](../developers).
