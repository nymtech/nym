---
title: "Encryption Standards Used in Nym"
description: "Cryptographic algorithms used across the Nym Network: Curve25519 key exchange, ChaCha20-Poly1305, AES-GCM-SIV, Lioness wide-block encryption, Noise protocol, and post-quantum KEM."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Encryption Standards

This page documents the cryptographic algorithms used throughout the Nym Network.

## Key exchange

All key exchanges use **Curve25519** via X25519. This elliptic curve provides 128-bit security with fast, constant-time implementations and compact 32-byte keys. Nym uses it for Sphinx packet key derivation (ECDH with each hop), Gateway authentication, WireGuard tunnel handshakes, and session key establishment.

Digital signatures use **Ed25519**, the signature scheme built on Curve25519. Node identity keys, client authentication, and QUIC TLS certificate verification all use Ed25519 signatures.

## Authenticated encryption

**ChaCha20-Poly1305** is the primary authenticated encryption scheme. It encrypts all WireGuard data packets in dVPN mode (via the `boringtun` and `wireguard-go` implementations), and is used in the Noise protocol handshakes and the OutFox packet format. It provides 256-bit security with authentication and performs well on devices without AES hardware acceleration.

**AES-GCM-SIV-256** is used for Gateway-client shared key encryption (protocol version 3+). The SIV (Synthetic Initialization Vector) construction degrades gracefully if a nonce is accidentally reused — important in distributed systems where nonce management is harder.

**AES-CTR-128** is used in Sphinx header encryption, where the stream cipher combines with blinding factors to create the layered encryption that each mix node peels away.

## Node authentication

The **Noise protocol** framework (via the `snow` crate) provides authenticated key exchange between nodes. Two cipher suites are in use:

- `Noise_XKpsk3_25519_AESGCM_SHA256`
- `Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s`

These provide mutual authentication, forward secrecy, and resistance to key-compromise impersonation.

## Wide-block encryption

**Lioness** is a wide-block cipher used for Sphinx packet payloads. It's constructed from ChaCha20 and BLAKE2, encrypting the entire payload as a single block. This property is essential for Sphinx: modifying any part of the payload invalidates the entire payload, preventing certain manipulation attacks.

The Lioness implementation is part of the external [`sphinx-packet`](https://github.com/nymtech/sphinx) crate used by Nym.

## Hashing

**BLAKE2** variants are used in the WireGuard Noise handshake (BLAKE2s) and in Lioness payload encryption (BLAKE2b via the sphinx-packet crate).

**BLAKE3** is used for modern key derivation in the KKT protocol and data observatory components.

**SHA-256** and **SHA-512** appear where compatibility with Cosmos SDK, HKDF, and standard tooling is required.

## Key derivation

**HKDF** (HMAC-based Key Derivation Function, RFC 5869) derives session keys from shared secrets. Both HKDF-SHA-256 and HKDF-SHA-512 variants are used, with HKDF-SHA-512 as the primary variant for `DerivationMaterial` in the SDK.

**Argon2** is used for password-based key derivation when protecting locally stored keys and credentials.

## Wallet cryptography

**Secp256k1** (via the `k256` crate) and **ECDSA** handle transaction signing and key management for the Nyx blockchain, consistent with Cosmos SDK conventions. **BIP32** hierarchical deterministic key derivation supports hardware wallet integration via Ledger.

## zk-nym cryptography

The credential system uses **BLS12-381**, a pairing-friendly elliptic curve that enables threshold signatures, signature aggregation, and zero-knowledge proofs. The Nym API Quorum uses BLS for distributed key generation and threshold blind signatures.

**Pedersen commitments** hide attribute values in credentials while allowing verification. **Zero-knowledge proofs** enable selective disclosure — proving properties about credentials without revealing the credentials themselves.

## Post-quantum cryptography (in progress)

The classical algorithms used today (Curve25519, BLS12-381) would be vulnerable to a sufficiently powerful quantum computer. Work is underway in the **KKT** (Key KEM Transport) module to add hybrid post-quantum key encapsulation using two NIST-standardised or finalist algorithms:

- **ML-KEM** (formerly CRYSTALS-Kyber) — a lattice-based KEM, now a NIST standard (FIPS 203)
- **Classic McEliece** — a code-based KEM with decades of cryptanalysis behind it

Both are available via the `libcrux` cryptographic library. The hybrid construction pairs these with classical X25519, so the system remains secure even if one primitive is broken. Post-quantum support will ship as part of the Lewes Protocol, which is currently in development.

## References

- [Sphinx paper](https://cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf) — Original Sphinx specification
- [Coconut paper](https://arxiv.org/pdf/1802.07344) — Credential scheme foundation
- [Offline Ecash paper](https://arxiv.org/pdf/2303.08221) — Compact ecash construction
- [WireGuard protocol](https://www.wireguard.com/protocol/) — dVPN tunnel specification
- [Noise protocol](http://www.noiseprotocol.org/) — Authenticated key exchange framework
- [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf) — Full protocol description
- [Nym Trust Center: Cryptography](https://nym.com/trust-center/cryptography) — Up-to-date cryptographic overview
