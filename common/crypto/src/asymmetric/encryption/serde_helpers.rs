// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::PublicKey;

pub mod bs58_x25519_pubkey {
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

pub mod option_bs58_x25519_pubkey {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        key: &Option<PublicKey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        key.map(|key| key.to_base58_string()).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<PublicKey>, D::Error> {
        let s = Option::<String>::deserialize(deserializer)?;
        s.map(|s| PublicKey::from_base58_string(&s).map_err(serde::de::Error::custom))
            .transpose()
    }
}
