# Cryptography

The Nym Network relies on several cryptographic systems working together. This section covers the algorithms, packet formats, and credential systems that provide privacy guarantees.

## Defense in depth

There isn't a single cryptographic scheme protecting traffic — transport encryption secures connections between nodes, Sphinx packets add per-hop encryption so each node only learns where to forward rather than the full route, the payload itself is encrypted end-to-end, and zk-nyms keep payment separate from usage.

## What's covered

[Encryption Standards](/network/cryptography/encryption-standards) documents the specific algorithms used throughout the network—Curve25519 for key exchange, AES and ChaCha20 for symmetric encryption, Lioness for wide-block encryption in Sphinx payloads.

[Sphinx Packets](/network/cryptography/sphinx) explains the packet format that enables layered encryption and anonymous routing. Each Sphinx packet contains routing information encrypted in layers, where each hop can only decrypt its own layer.

[zk-nyms](/network/cryptography/zk-nym) covers the anonymous credential system that separates payment from usage. This is how you can pay for network access without that payment being linkable to your activity.
