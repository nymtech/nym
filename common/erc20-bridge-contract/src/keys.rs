// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Serializable structures for what we find in common/crypto
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl AsRef<[u8]> for PublicKey {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Signature([u8; 32], [u8; 32]);

impl Signature {
    pub fn new(bytes: [u8; 64]) -> Self {
        let mut sig1 = [0u8; 32];
        let mut sig2 = [0u8; 32];
        sig1.copy_from_slice(&bytes[..32]);
        sig2.copy_from_slice(&bytes[32..]);

        Signature(sig1, sig2)
    }
    pub fn as_bytes(&self) -> [u8; 64] {
        let mut res = [0u8; 64];
        res[..32].copy_from_slice(&self.0);
        res[32..].copy_from_slice(&self.1);
        res
    }
}
