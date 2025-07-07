// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(from = "u8", into = "u8")]
pub enum NoiseVersion {
    V1,
    V2,
    Unknown(u8), //Implies a newer version we don't know
}

impl From<u8> for NoiseVersion {
    fn from(value: u8) -> Self {
        match value {
            1 => NoiseVersion::V1,
            2 => NoiseVersion::V1,
            other => NoiseVersion::Unknown(other),
        }
    }
}

impl From<NoiseVersion> for u8 {
    fn from(version: NoiseVersion) -> Self {
        match version {
            NoiseVersion::V1 => 1,
            NoiseVersion::V2 => 2,
            NoiseVersion::Unknown(other) => other,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
pub struct VersionedNoiseKey {
    #[schemars(with = "u8")]
    #[schema(value_type = u8)]
    pub supported_version: NoiseVersion,

    #[schemars(with = "String")]
    #[serde(with = "bs58_x25519_pubkey")]
    #[schema(value_type = String)]
    pub x25519_pubkey: x25519::PublicKey,
}
