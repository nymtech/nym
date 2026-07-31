// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::DirectoryTypesError;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SphinxKeys {
    /// key rotation id -> sphinx (x25519) public key bytes
    #[prost(btree_map = "uint32, bytes", tag = "1")]
    pub keys: BTreeMap<u32, Vec<u8>>,
}

impl SphinxKeys {
    /// Returns key for the particular key rotation. Returns Error if the encoded bytes are not 32 bytes long,
    /// e.g. length of an x25519 public key.
    pub fn key_for_rotation(
        &self,
        rotation_id: u32,
    ) -> Option<Result<[u8; 32], DirectoryTypesError>> {
        let key_bytes = self.keys.get(&rotation_id)?;

        if key_bytes.len() != 32 {
            return Some(Err(DirectoryTypesError::InvalidSphinxKeyLength {
                got: key_bytes.len(),
            }));
        }

        let mut k = [0u8; 32];
        // SAFETY: we just checked the length is 32 bytes
        k.copy_from_slice(key_bytes);
        Some(Ok(k))
    }
}
