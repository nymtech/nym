// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct Mixnode {
    // /// Base58 encoded ed25519 EdDSA public key of the mixnode.
    // pub encoded_identity_key: String,
    //
    // /// Base58-encoded x25519 public key used for sphinx key derivation.
    // pub encoded_sphinx_key: String,
}
