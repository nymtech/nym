# Cryptography

The Nym Network relies on several cryptographic systems working together. This section covers the algorithms, packet formats, and credential systems that provide privacy guarantees.

## Defense in depth

Multiple cryptographic layers protect user privacy. Transport encryption secures all network connections. Sphinx packets provide per-hop encryption so each node only sees its next destination. End-to-end encryption protects payload contents. And the zk-nym credential system ensures payment cannot be linked to usage.

## What's covered

[Encryption Standards](/network/cryptography/encryption-standards) documents the specific algorithms used throughout the network—Curve25519 for key exchange, AES and ChaCha20 for symmetric encryption, Lioness for wide-block encryption in Sphinx payloads.

[Sphinx Packets](/network/cryptography/sphinx) explains the packet format that enables layered encryption and anonymous routing. Each Sphinx packet contains routing information encrypted in layers, where each hop can only decrypt its own layer.

[zk-nyms](/network/cryptography/zk-nym) covers the anonymous credential system that separates payment from usage. This is how you can pay for network access without that payment being linkable to your activity.
