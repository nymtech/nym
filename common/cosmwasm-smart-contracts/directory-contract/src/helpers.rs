// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Canonical encodings shared between the directory contract and off-chain clients
//! (so a client can reproduce the exact bytes the contract signs and hashes).
//! `node_id` is always encoded big-endian, matching the storage key's ordering.

use crate::Namespace;
use nym_mixnet_contract_common::NodeId;

/// Append `bytes` prefixed with its u64 little-endian length, so adjacent
/// variable-length fields cannot be confused with one another.
fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// The exact bytes a node signs (and the contract verifies via `ed25519_verify`)
/// for a node-entry write or delete. Binding `node_id`, `label`, and `sequence`
/// means a signature cannot be replayed or moved to another `(node_id, label)`.
pub fn node_signing_payload(node_id: NodeId, label: &str, sequence: u64, data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&node_id.to_be_bytes());
    push_len_prefixed(&mut buf, label.as_bytes());
    buf.extend_from_slice(&sequence.to_le_bytes());
    push_len_prefixed(&mut buf, data);
    buf
}

/// The canonical leaf folded into the global LtHash digest. The namespace tag and
/// length-prefixing keep leaves unambiguous across key-classes. `id` is the
/// big-endian `node_id` for the node namespace, or the raw id bytes otherwise.
pub fn digest_leaf(namespace: Namespace, id: &[u8], label: &str, value: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(namespace.tag());
    push_len_prefixed(&mut buf, id);
    push_len_prefixed(&mut buf, label.as_bytes());
    push_len_prefixed(&mut buf, value);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_payload_is_deterministic_and_field_sensitive() {
        let base = node_signing_payload(7, "description", 3, b"data");
        assert_eq!(base, node_signing_payload(7, "description", 3, b"data"));
        assert_ne!(base, node_signing_payload(8, "description", 3, b"data"));
        assert_ne!(base, node_signing_payload(7, "network", 3, b"data"));
        assert_ne!(base, node_signing_payload(7, "description", 4, b"data"));
        assert_ne!(base, node_signing_payload(7, "description", 3, b"data2"));
    }

    #[test]
    fn signing_payload_length_prefix_disambiguates() {
        // (label "ab", data "c") and (label "a", data "bc") must not collide
        assert_ne!(
            node_signing_payload(1, "ab", 0, b"c"),
            node_signing_payload(1, "a", 0, b"bc"),
        );
    }

    #[test]
    fn digest_leaf_namespace_separates_classes() {
        let node = digest_leaf(Namespace::Node, &1u32.to_be_bytes(), "x", b"v");
        let curated = digest_leaf(Namespace::Curated, &1u32.to_be_bytes(), "x", b"v");
        assert_ne!(node, curated, "namespace tag must separate the classes");
    }

    #[test]
    fn digest_leaf_length_prefix_disambiguates() {
        // (id "ab", label "c") vs (id "a", label "bc") - same namespace and value
        assert_ne!(
            digest_leaf(Namespace::Node, b"ab", "c", b""),
            digest_leaf(Namespace::Node, b"a", "bc", b""),
        );
    }
}
