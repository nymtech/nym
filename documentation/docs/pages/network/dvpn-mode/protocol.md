# dVPN Protocol

This page covers the technical details of dVPN mode's protocol stack and encryption.

TODO CHECK UPDATES AGAINST CONFLUENCE DESIGN DOCS

## Protocol layers

dVPN mode combines WireGuard with additional layer encryption. The client-to-Entry Gateway connection uses WireGuard, providing fast handshakes, efficient encryption, and graceful reconnection. The Entry-to-Exit Gateway connection adds another encryption layer using AES-GCM-SIV-256.

```
┌─────────────────────────────────────────┐
│           Application Data              │
├─────────────────────────────────────────┤
│    Layer Encryption (Entry → Exit)      │
├─────────────────────────────────────────┤
│    WireGuard (Client → Entry)           │
├─────────────────────────────────────────┤
│              UDP/IP                     │
└─────────────────────────────────────────┘
```

## Encryption

The WireGuard layer uses Curve25519 for key exchange, ChaCha20-Poly1305 for symmetric encryption, and BLAKE2s for hashing. This provides 256-bit security with modern, well-audited primitives.

WireGuard integration: [`nym-vpn-core/crates/nym-wg-go`](https://github.com/nymtech/nym-vpn-client/tree/main/nym-vpn-core/crates/nym-wg-go)

The inner layer uses AES-GCM-SIV-256, an authenticated encryption scheme with nonce-misuse resistance. Even if a nonce is accidentally reused, the scheme degrades gracefully rather than catastrophically. Keys are derived through ECDH between the client and Exit Gateway, with separate keys for each direction.

AEAD implementation: [`common/crypto/src/symmetric/aead.rs`](https://github.com/nymtech/nym/blob/develop/common/crypto/src/symmetric/aead.rs)

## Packet format

All packets are padded to a uniform size before encryption. The packet contains a header with routing information, the encrypted payload (original data plus padding), and an authentication tag. Padding is removed at the Exit Gateway before forwarding to the destination.

This uniformity matters because packet sizes can leak information about content types—video streams have different size patterns than text messages. With uniform packets, this side channel is eliminated.

## Connection lifecycle

When connecting, the client first selects Entry and Exit Gateways based on latency, location preference, or random selection. It then presents a zk-nym credential to the Entry Gateway for anonymous authentication. The credential proves payment without revealing identity—it's re-randomized for each connection and cannot be linked to previous usage.

Once authenticated, the client establishes a WireGuard tunnel to the Entry Gateway, which establishes a link to the Exit Gateway. Traffic then flows through both hops until the session ends.

VPN connection handling: [`nym-vpn-core/crates/nym-vpn-lib`](https://github.com/nymtech/nym-vpn-client/tree/main/nym-vpn-core/crates/nym-vpn-lib)

## Security properties

The protocol provides forward secrecy—new session keys are derived for each connection, so compromising long-term keys doesn't expose past sessions. WireGuard's key rotation provides additional forward secrecy within sessions.

The split-knowledge architecture ensures the Entry Gateway knows your IP but not your destinations or payload content, while the Exit Gateway knows your destinations but not your IP. Neither can correlate the two.

Replay protection comes from WireGuard's counter-based mechanism and from zk-nym serial numbers that prevent credential reuse.

## Relationship to mixnet mode

dVPN mode shares infrastructure with mixnet mode. Both use the same Entry and Exit Gateways, the same credential system, and the same packet sizes. External observers cannot distinguish between the two modes. The difference is internal: mixnet mode routes through three additional Mix Node layers with delays and cover traffic, while dVPN mode routes directly between gateways.

This shared infrastructure means improvements to Gateways and credentials benefit both modes, and the indistinguishability between modes provides privacy benefits even for dVPN users.
