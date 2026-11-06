// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// (De)serializes `KEMKeysDigests` as `{ kem: { hash_function: hex_digest } }`
pub mod hex_kem_keys_digests {
    use crate::KEMKeysDigests;
    use serde::{Deserializer, Serializer};
    use serde_with::{DeserializeAs, DisplayFromStr, SerializeAs, hex::Hex};
    use std::collections::BTreeMap;

    type Repr = BTreeMap<DisplayFromStr, BTreeMap<DisplayFromStr, Hex>>;

    pub fn serialize<S: Serializer>(
        digests: &KEMKeysDigests,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Repr::serialize_as(digests, serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<KEMKeysDigests, D::Error> {
        Repr::deserialize_as(deserializer)
    }
}
