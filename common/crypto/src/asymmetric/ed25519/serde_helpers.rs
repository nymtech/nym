// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::PublicKey;

pub mod bs58_ed25519_pubkey {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(key: &PublicKey, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&key.to_base58_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PublicKey, D::Error> {
        let s = String::deserialize(deserializer)?;
        PublicKey::from_base58_string(s).map_err(serde::de::Error::custom)
    }
}

pub mod bs58_ed25519_signature {
    use crate::asymmetric::ed25519::Signature;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        signature: &Signature,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&signature.to_base58_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Signature, D::Error> {
        let s = String::deserialize(deserializer)?;
        Signature::from_base58_string(s).map_err(serde::de::Error::custom)
    }
}
