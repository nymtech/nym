// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Canonical encodings shared between the directory contract and off-chain clients
//! (so a client can reproduce the exact bytes the contract signs and hashes).
//! `node_id` is always encoded big-endian, matching the storage key's ordering.

use crate::DirectoryContractError;
use nym_mixnet_contract_common::NodeId;

/// Append `bytes` prefixed with its u64 little-endian length, so adjacent
/// variable-length fields cannot be confused with one another. Shared with
/// [`crate::EntryKey`]'s storage-key / digest-leaf encoders.
pub(crate) fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// A forward cursor over an encoded entry value, the read counterpart to
/// [`push_len_prefixed`]. Used by the `try_from_bytes` value codecs; every short
/// read is reported as [`DirectoryContractError::MalformedEntryValue`].
pub(crate) struct ValueReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ValueReader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        ValueReader { buf, pos: 0 }
    }

    /// Consume and return the next `n` bytes, erroring if fewer remain.
    fn take(&mut self, n: usize) -> Result<&'a [u8], DirectoryContractError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|end| *end <= self.buf.len())
            .ok_or_else(|| {
                DirectoryContractError::MalformedEntryValue("unexpected end of value".to_owned())
            })?;
        let out = &self.buf[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    /// Read a fixed 8-byte little-endian `u64`.
    pub(crate) fn read_u64_le(&mut self) -> Result<u64, DirectoryContractError> {
        let bytes = self.take(8)?;
        // SAFETY: `take` returned exactly 8 bytes.
        #[allow(clippy::unwrap_used)]
        Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
    }

    /// Read a [`push_len_prefixed`]-framed slice (u64 LE length, then that many bytes).
    pub(crate) fn read_len_prefixed(&mut self) -> Result<&'a [u8], DirectoryContractError> {
        let len = self.read_u64_le()? as usize;
        self.take(len)
    }

    /// The remaining, unframed bytes - the final field of a value layout.
    pub(crate) fn rest(self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
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
