// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{PrivateKey, PublicKey};

pub mod bs58_x25519_private_key {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(key: &PrivateKey, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&key.to_base58_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PrivateKey, D::Error> {
        let s = String::deserialize(deserializer)?;
        PrivateKey::from_base58_string(s).map_err(serde::de::Error::custom)
    }
}

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
        match Option::<String>::deserialize(deserializer)? {
            None => Ok(None),
            Some(s) => {
                if s.is_empty() {
                    Ok(None)
                } else {
                    Some(PublicKey::from_base58_string(&s).map_err(serde::de::Error::custom))
                        .transpose()
                }
            }
        }
    }
}

#[cfg(feature = "libcrux_x25519")]
pub mod bs58_dh_public_key {
    use crate::asymmetric::x25519;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        key: &libcrux_psq::handshake::types::DHPublicKey,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let x25519: x25519::PublicKey = (*key).into();
        serializer.serialize_str(&x25519.to_base58_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<libcrux_psq::handshake::types::DHPublicKey, D::Error> {
        let s = String::deserialize(deserializer)?;
        let x25519 = x25519::PublicKey::from_base58_string(s).map_err(serde::de::Error::custom)?;
        Ok(x25519.into())
    }
}
