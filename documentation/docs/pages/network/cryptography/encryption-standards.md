# Encryption Standards

This page documents the cryptographic algorithms used throughout the Nym Network.

## Overview

| Algorithm | Type | Usage |
|-----------|------|-------|
| AES-GCM-SIV-256 | Authenticated encryption | Payload encryption with nonce-misuse resistance |
| AES-CTR-128 | Stream cipher | Sphinx header encryption |
| ChaCha20-Poly1305 | Authenticated encryption | WireGuard tunnels (dVPN mode) |
| Lioness | Wide-block cipher | Sphinx payload encryption |
| Curve25519 | Elliptic curve | Key exchange (X25519) and signatures (Ed25519) |
| BLS12-381 | Pairing-friendly curve | zk-nym threshold signatures |
| BLAKE2 | Hash function | Key derivation, checksums |

## Symmetric Encryption

### AES-GCM-SIV-256

**Usage**: Primary authenticated encryption for payloads

AES-GCM-SIV provides:
- 256-bit key security
- Authenticated encryption with associated data (AEAD)
- Nonce-misuse resistance (safety even with repeated nonces)

The SIV (Synthetic Initialization Vector) mode is critical for mixnet applications where nonce reuse risks are higher due to the distributed nature of the system.

### AES-CTR-128

**Usage**: Sphinx header encryption

AES in Counter (CTR) mode provides:
- Stream cipher operation
- 128-bit key security
- Efficient parallel processing

Used in Sphinx packet headers where the block cipher is combined with a blinding factor.

### ChaCha20-Poly1305

**Usage**: WireGuard tunnels in dVPN mode

ChaCha20-Poly1305 provides:
- 256-bit key security
- Authenticated encryption
- High performance on devices without AES hardware acceleration

This is the standard cipher suite for WireGuard and is used for the client-to-Entry Gateway tunnel in dVPN mode.

### Lioness

**Usage**: Sphinx packet payload encryption

Lioness is a wide-block cipher constructed from:
- A stream cipher (ChaCha20)
- A hash function (BLAKE2)

Properties:
- Encrypts entire payload as single block
- Any bit change affects entire ciphertext
- Prevents partial payload manipulation

The wide-block property is essential for Sphinx: it ensures that modifying any part of the payload invalidates the entire payload, preventing certain manipulation attacks.

## Asymmetric Cryptography

### Curve25519 (X25519)

**Usage**: Key exchange

X25519 provides:
- 128-bit security level
- Fast, constant-time implementation
- Small key size (32 bytes)

Used for:
- Sphinx packet key derivation (ECDH with each hop)
- Gateway authentication
- Session key establishment

### Ed25519

**Usage**: Digital signatures

Ed25519 provides:
- 128-bit security level
- Fast signature generation and verification
- Deterministic signatures (no random number needed)

Used for:
- Node identity verification
- zk-nym credential signatures (base scheme)
- Message authentication

### BLS12-381

**Usage**: zk-nym threshold signatures

BLS12-381 is a pairing-friendly elliptic curve providing:
- Efficient threshold signature schemes
- Signature aggregation
- Zero-knowledge proof support

Used in the zk-nym system for:
- Distributed key generation among the Quorum
- Threshold blind signatures
- Credential verification

## Hash Functions

### BLAKE2

**Usage**: General-purpose hashing

BLAKE2 variants used:
- **BLAKE2b**: For general hashing (up to 64 bytes output)
- **BLAKE2s**: For shorter hashes (up to 32 bytes output)

Properties:
- Faster than SHA-3 and SHA-2
- Security equivalent to SHA-3
- Built-in keying mode for MACs

### SHA-256

**Usage**: Blockchain compatibility

Used where compatibility with Cosmos SDK and standard tooling is required.

## Key Derivation

### HKDF

HMAC-based Key Derivation Function (RFC 5869) is used for:
- Deriving session keys from shared secrets
- Expanding key material for multiple purposes
- Domain separation between different key uses

## Protocol-Specific Usage

### Sphinx Packet Processing

At each hop, the following cryptographic operations occur:

1. **ECDH**: Compute shared secret with node's public key
2. **HKDF**: Derive encryption key and blinding factor
3. **AES-CTR**: Decrypt header to reveal routing info
4. **HMAC**: Verify header integrity
5. **Lioness**: Re-encrypt payload (maintains unlinkability)
6. **Blind**: Update curve point for next hop

### dVPN Mode

The dVPN mode layers:

1. **WireGuard layer** (ChaCha20-Poly1305): Client ↔ Entry Gateway
2. **Inner layer** (AES-GCM-SIV): Entry Gateway ↔ Exit Gateway
3. **Packet padding**: Uniform size for all packets

### zk-nym Credentials

The credential system uses:

1. **BLS signatures**: For threshold issuance
2. **Pedersen commitments**: For attribute hiding
3. **Zero-knowledge proofs**: For selective disclosure
4. **Re-randomization**: For unlinkability

## Security Considerations

### Key Rotation

- Node keys should be rotated periodically
- Session keys are ephemeral by design
- Credential keys use threshold distribution

### Side-Channel Resistance

- Constant-time implementations used where available
- Timing attacks mitigated at protocol level
- Memory-safe Rust implementations

### Quantum Considerations

Current algorithms are not post-quantum secure. Research is ongoing into:
- Post-quantum Sphinx variants
- Lattice-based credential schemes
- Hybrid classical/post-quantum approaches

## References

- [Sphinx Paper](https://cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf): Original Sphinx specification
- [Coconut Paper](https://arxiv.org/pdf/1802.07344): Credential scheme foundation
- [WireGuard Protocol](https://www.wireguard.com/protocol/): dVPN tunnel specification
- [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf): Full protocol description
