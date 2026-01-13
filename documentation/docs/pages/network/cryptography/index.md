# Cryptography

The Nym Network relies on several cryptographic systems working together to provide its privacy guarantees.

## Overview

| System | Purpose |
|--------|---------|
| [Encryption Standards](./encryption-standards) | Algorithms used throughout the network |
| [Sphinx Packets](./sphinx) | Packet format for layered encryption |
| [zk-nyms](./zk-nym) | Anonymous credentials for access control |

## Core Principles

### Defense in Depth

Multiple cryptographic layers protect user privacy:

1. **Transport encryption**: All network connections are encrypted
2. **Packet encryption**: Sphinx format with per-hop encryption
3. **Payload encryption**: End-to-end encryption of message content
4. **Credential encryption**: zk-nyms protect payment privacy

### Modern Primitives

The Nym Network uses well-vetted, modern cryptographic primitives:

- **Curve25519**: For key exchange (X25519) and signatures (Ed25519)
- **AES**: For symmetric encryption in various modes
- **ChaCha20-Poly1305**: For authenticated encryption
- **Lioness**: Wide-block cipher for Sphinx payloads
- **BLS12-381**: For zk-nym threshold signatures

### Zero-Knowledge Foundations

The zk-nym credential system enables proving statements without revealing underlying data:

- Proof of payment without revealing payment details
- Credential validity without linkability
- Access rights without identity disclosure

## Key Properties

### Unlinkability

Cryptographic design ensures that:
- Successive packets from the same user cannot be linked
- Credential usage cannot be linked to credential issuance
- Sessions cannot be correlated across time

### Forward Secrecy

Key management provides forward secrecy:
- Session keys are ephemeral
- Long-term key compromise doesn't expose past traffic
- Key rotation limits exposure window

### Verifiable Integrity

All packets include integrity verification:
- HMACs at each Sphinx layer
- Authentication tags in encrypted payloads
- Digital signatures on credentials

## Further Reading

- [Encryption Standards](./encryption-standards): Detailed algorithm specifications
- [Sphinx Packet Format](./sphinx): How packets are constructed and processed
- [zk-nyms](./zk-nym): Anonymous credential system deep dive
