// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::IdentityKey;

pub struct NymNode {
    /// Network address of this mixnode, for example 1.1.1.1 or foo.nymnode.com
    /// that will used for discovering other capabilities of this node.
    pub host: String,

    /// Base58-encoded ed25519 EdDSA public key.
    pub identity_key: IdentityKey,
}
