// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct IpPacketRouter {
    /// Base58 encoded ed25519 EdDSA public key of the ip-packet-router.
    pub encoded_identity_key: String,

    /// Base58-encoded x25519 public key used for performing key exchange with remote clients.
    pub encoded_x25519_key: String,

    /// Nym address of this ip packet router.
    pub address: String,
}
