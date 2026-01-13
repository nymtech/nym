# Sphinx Packets

Sphinx is the cryptographic packet format used for all mixnet traffic. It provides layered encryption where each hop can only decrypt its own routing information, ensuring that no single node knows both the source and destination of a packet.

## How Sphinx works

When a client sends a message through the mixnet, it constructs a Sphinx packet with multiple encryption layers—one for each hop in the route. The outermost layer is encrypted for the first hop (Entry Gateway), the next layer for the second hop (Mix Node Layer 1), and so on until the innermost layer contains the actual payload encrypted for the recipient.

At each hop, the node uses its private key to decrypt its layer, revealing the address of the next hop and a new Sphinx packet to forward. The node cannot see any other routing information or the payload contents.

Implementation: [`common/nymsphinx`](https://github.com/nymtech/nym/tree/develop/common/nymsphinx)

## Packet structure

All Sphinx packets have a fixed payload size of 2048 bytes. This uniformity is critical—if packets varied in size, nodes could infer their position in the route or correlate packets by size.

The packet contains a header with encrypted routing information for each hop, HMACs to verify integrity at each layer, and the encrypted payload. The header uses a clever "onion" structure where processing at each hop reveals only the next hop's information while maintaining constant size through padding.

Types and constants: [`common/nymsphinx/types`](https://github.com/nymtech/nym/tree/develop/common/nymsphinx/types)

Packet preparation: [`common/nymsphinx/src/preparer`](https://github.com/nymtech/nym/tree/develop/common/nymsphinx/src/preparer)

## Integrity verification

Each layer includes an HMAC (Hash-based Message Authentication Code) that the receiving node verifies before processing. This prevents malicious nodes from modifying packet contents en route. If the HMAC doesn't match, the packet is dropped.

The payload uses Lioness wide-block encryption, which means any modification to any part of the payload invalidates the entire payload. This prevents bit-flipping attacks where an adversary might try to modify specific bytes.

## Key derivation

For each hop, the client performs an ECDH key exchange using the node's public key and an ephemeral key embedded in the packet header. This shared secret is then used with HKDF to derive the symmetric keys for that layer's encryption and HMAC.

The ephemeral key is "blinded" at each hop so that successive nodes cannot correlate packets by the key value. Each node sees a different ephemeral key even though they're mathematically related.

Shared key derivation: [`common/crypto/src/shared_key.rs`](https://github.com/nymtech/nym/blob/develop/common/crypto/src/shared_key.rs)

## Message fragmentation

Messages larger than a single Sphinx payload are split into fragments. Each fragment travels independently through the network, potentially taking different routes. The recipient reassembles the fragments into the original message.

Chunking implementation: [`common/nymsphinx/chunking`](https://github.com/nymtech/nym/tree/develop/common/nymsphinx/chunking)

## External implementation

Nym uses the [`sphinx-packet`](https://github.com/nymtech/sphinx) crate for core Sphinx operations. This crate handles packet construction, header encryption, layer processing, and the mathematical operations for key blinding.

## References

- [Sphinx paper](https://cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf) — Original specification and security proofs
- [Elle Mouton's Sphinx explainer](https://ellemouton.com/posts/sphinx/) — Detailed walkthrough of packet construction
- [Nym Whitepaper §4](https://nym.com/nym-whitepaper.pdf) — Sphinx in the context of Nym
