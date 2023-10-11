// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ClientMessage {
    Initial(InitMessage),
    Final(Client),
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InitMessage {
    /// Base64 encoded x25519 public key
    pub pub_key: String,
}

/// Client that wants to register sends its PublicKey and SocketAddr bytes mac digest encrypted with a DH shared secret.
/// Gateway/Nym node can then verify pub_key payload using the same process
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Client {
    /// Base64 encoded x25519 public key
    pub pub_key: String,

    /// Client's socket address
    #[cfg_attr(feature = "openapi", schema(example = "1.2.3.4:51820"))]
    pub socket: String,

    /// Sha256 hmac on the data (alongside the prior nonce)
    pub mac: String,
}
