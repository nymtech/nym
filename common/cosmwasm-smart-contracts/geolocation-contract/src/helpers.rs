// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Canonical encodings shared between the geolocation contract and off-chain clients, so a
//! client can reproduce the exact bytes the contract hashes.

/// Append `bytes` prefixed with its u32 little-endian length, so adjacent variable-length
/// fields cannot be confused with one another. Same convention as the directory contract's
/// leaf encoder.
pub(crate) fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}
