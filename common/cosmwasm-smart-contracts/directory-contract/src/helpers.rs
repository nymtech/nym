// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Canonical encodings shared between the directory contract and off-chain clients
//! (so a client can reproduce the exact bytes the contract signs and hashes).
//! `node_id` is always encoded big-endian, matching the storage key's ordering.

use crate::DirectoryContractError;
use nym_mixnet_contract_common::NodeId;

/// Append `bytes` prefixed with its u32 little-endian length, so adjacent
/// variable-length fields cannot be confused with one another. Shared with
/// [`crate::EntryKey`]'s storage-key / digest-leaf encoders.
pub(crate) fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

pub(crate) fn read_len_prefixed(buf: &[u8]) -> Result<&[u8], DirectoryContractError> {
    if buf.len() < 4 {
        return Err(DirectoryContractError::MalformedEntryValue(
            "unexpected end of value".to_owned(),
        ));
    }
    let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let end = 4 + len as usize;
    if buf.len() < end {
        return Err(DirectoryContractError::MalformedEntryValue(
            "unexpected end of value".to_owned(),
        ));
    }
    Ok(&buf[4..end])
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
}
