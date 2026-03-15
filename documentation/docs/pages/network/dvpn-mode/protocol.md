---
title: "dVPN Protocol Stack and Encryption"
description: "Technical details of Nym dVPN mode's protocol layers, including WireGuard tunnels, AES-GCM-SIV-256 layer encryption, and packet format tradeoffs."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# dVPN Protocol

This page covers the technical details of dVPN mode's protocol stack and encryption.

## Protocol layers

dVPN mode combines WireGuard with additional layer encryption. The client-to-Entry Gateway connection uses WireGuard, providing fast handshakes, efficient encryption, and graceful reconnection. The Entry-to-Exit Gateway connection adds another encryption layer using AES-GCM-SIV-256.

```
+-----------------------------------------+
|           Application Data              |
+-----------------------------------------+
|    Layer Encryption (Entry -> Exit)      |
+-----------------------------------------+
|    WireGuard (Client -> Entry)           |
+-----------------------------------------+
|              UDP/IP                      |
+-----------------------------------------+
```

## Encryption

The WireGuard layer uses Curve25519 for key exchange, ChaCha20-Poly1305 for symmetric encryption, and BLAKE2s for hashing. This provides 256-bit security with modern, well-audited primitives.

The inner layer uses AES-GCM-SIV-256, an authenticated encryption scheme with nonce-misuse resistance. Even if a nonce is accidentally reused, the scheme degrades gracefully rather than catastrophically. Keys are derived through ECDH between the client and Exit Gateway, with separate keys for each direction.

## Packet format

dVPN mode uses standard WireGuard packet framing — packets are not padded to a uniform size. This means packet sizes may vary and could in principle leak information about content types (video streams have different size patterns than text messages). This is a tradeoff: uniform padding would add overhead and reduce throughput, which conflicts with dVPN mode's goal of low-latency, high-throughput connectivity. For uniform packet sizes, use [mixnet mode](/network/mixnet-mode), which wraps all traffic in fixed-size Sphinx packets.

## Connection lifecycle

When connecting, the client first selects Entry and Exit Gateways based on latency, location preference, or random selection. It then presents a zk-nym credential to the Entry Gateway for anonymous authentication. The credential proves payment without revealing identity—it's re-randomized for each connection and cannot be linked to previous usage.

Once authenticated, the client establishes a WireGuard tunnel to the Entry Gateway, which establishes a link to the Exit Gateway. Traffic then flows through both hops until the session ends.

## Security properties

The protocol provides forward secrecy—new session keys are derived for each connection, so compromising long-term keys doesn't expose past sessions. WireGuard's key rotation provides additional forward secrecy within sessions.

The split-knowledge architecture ensures the Entry Gateway knows your IP but not your destinations or payload content, while the Exit Gateway knows your destinations but not your IP. Neither can correlate the two.

Replay protection comes from WireGuard's counter-based mechanism and from zk-nym serial numbers that prevent credential reuse.

## Relationship to mixnet mode

dVPN mode shares infrastructure with mixnet mode. Both use the same Entry and Exit Gateways and the same credential system. The difference is in how traffic is handled: mixnet mode routes through three additional Mix Node layers with delays and cover traffic using fixed-size Sphinx packets, while dVPN mode routes directly between gateways using WireGuard. The two modes are distinguishable at the protocol level due to their different packet formats and traffic patterns.

This shared infrastructure means improvements to Gateways and credentials benefit both modes.
