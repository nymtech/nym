// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, PartialEq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct PresharedKey([u8; 32]);

impl PresharedKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for PresharedKey {
    fn from(key: [u8; 32]) -> PresharedKey {
        PresharedKey(key)
    }
}

impl From<PresharedKey> for [u8; 32] {
    fn from(key: PresharedKey) -> [u8; 32] {
        key.0
    }
}
