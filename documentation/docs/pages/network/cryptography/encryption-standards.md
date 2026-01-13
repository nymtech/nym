# Encryption Standards

This page documents the cryptographic algorithms used throughout the Nym Network.

## Key exchange

All key exchanges use **Curve25519** via X25519. This elliptic curve provides 128-bit security with fast, constant-time implementations and compact 32-byte keys. Nym uses it for Sphinx packet key derivation (ECDH with each hop), Gateway authentication, and session key establishment.

Implementation: [`common/crypto/src/asymmetric/x25519`](https://github.com/nymtech/nym/tree/develop/common/crypto/src/asymmetric/x25519)

Digital signatures use **Ed25519**, the signature scheme built on Curve25519. It provides fast signature generation and verification with deterministic signatures that don't require a random number generator.

Implementation: [`common/crypto/src/asymmetric/ed25519`](https://github.com/nymtech/nym/tree/develop/common/crypto/src/asymmetric/ed25519)

## Symmetric encryption

**ChaCha20-Poly1305** is the primary authenticated encryption scheme, particularly for WireGuard tunnels in dVPN mode. It provides 256-bit security with authentication, and performs well on devices without AES hardware acceleration.

WireGuard integration: [`nym-vpn-core/crates/nym-wg-go`](https://github.com/nymtech/nym-vpn-client/tree/main/nym-vpn-core/crates/nym-wg-go)

**AES-GCM-SIV-256** is used where nonce-misuse resistance matters. The SIV (Synthetic Initialization Vector) construction degrades gracefully if a nonce is accidentally reused—important in distributed systems where nonce management is harder.

Implementation: [`common/crypto/src/symmetric/aead.rs`](https://github.com/nymtech/nym/blob/develop/common/crypto/src/symmetric/aead.rs)

**AES-CTR-128** is used in Sphinx header encryption, where the stream cipher combines with blinding factors.

Implementation: [`common/crypto/src/symmetric/stream_cipher.rs`](https://github.com/nymtech/nym/blob/develop/common/crypto/src/symmetric/stream_cipher.rs)

## Wide-block encryption

**Lioness** is a wide-block cipher used for Sphinx packet payloads. It's constructed from ChaCha20 and BLAKE2, encrypting the entire payload as a single block. This property is essential for Sphinx: modifying any part of the payload invalidates the entire payload, preventing certain manipulation attacks.

The Lioness implementation is part of the external [`sphinx-packet`](https://github.com/nymtech/sphinx) crate used by Nym.

## Hashing

**BLAKE2** variants are used for general-purpose hashing and key derivation. BLAKE2b handles longer outputs up to 64 bytes; BLAKE2s handles shorter outputs up to 32 bytes. Both are faster than SHA-2 and SHA-3 with equivalent security.

Implementation: [`common/crypto/src/hmac.rs`](https://github.com/nymtech/nym/blob/develop/common/crypto/src/hmac.rs) (HMAC with BLAKE3)

**SHA-256** appears where compatibility with Cosmos SDK and standard tooling is required.

## Key derivation

**HKDF** (HMAC-based Key Derivation Function, RFC 5869) derives session keys from shared secrets, expands key material for multiple purposes, and provides domain separation between different key uses.

Implementation: [`common/crypto/src/hkdf.rs`](https://github.com/nymtech/nym/blob/develop/common/crypto/src/hkdf.rs)

Shared key derivation combining X25519 ECDH with HKDF: [`common/crypto/src/shared_key.rs`](https://github.com/nymtech/nym/blob/develop/common/crypto/src/shared_key.rs)

## zk-nym cryptography

The credential system uses **BLS12-381**, a pairing-friendly elliptic curve that enables threshold signatures, signature aggregation, and zero-knowledge proofs. The Nym API Quorum uses BLS for distributed key generation and threshold blind signatures.

Implementation: [`common/nym_offline_compact_ecash`](https://github.com/nymtech/nym/tree/develop/common/nym_offline_compact_ecash)

Key generation and setup: [`scheme/keygen.rs`](https://github.com/nymtech/nym/blob/develop/common/nym_offline_compact_ecash/src/scheme/keygen.rs)

**Pedersen commitments** hide attribute values in credentials while allowing verification. **Zero-knowledge proofs** enable selective disclosure—proving properties about credentials without revealing the credentials themselves.

Zero-knowledge proofs: [`proofs/`](https://github.com/nymtech/nym/tree/develop/common/nym_offline_compact_ecash/src/proofs)

## Quantum considerations

Current algorithms are not post-quantum secure. Curve25519, AES, and BLS would all be vulnerable to a sufficiently powerful quantum computer. Research is ongoing into post-quantum Sphinx variants and lattice-based credential schemes. For now, the network provides strong security against classical computers.

## References

- [Sphinx paper](https://cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf) — Original Sphinx specification
- [Coconut paper](https://arxiv.org/pdf/1802.07344) — Credential scheme foundation
- [Offline Ecash paper](https://arxiv.org/pdf/2303.08221) — Compact ecash construction
- [WireGuard protocol](https://www.wireguard.com/protocol/) — dVPN tunnel specification
- [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf) — Full protocol description
