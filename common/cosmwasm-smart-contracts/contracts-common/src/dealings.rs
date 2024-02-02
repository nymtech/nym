// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};

// some sane upper-bound size on byte sizes
// currently set to 128 bytes
pub const MAX_DISPLAY_SIZE: usize = 128;

// helps to transfer bytes between contract boundary to decrease amount of data sent accross
// after it's put to `Binary`
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, JsonSchema)]
pub struct ContractSafeBytes(pub Vec<u8>);

impl Deref for ContractSafeBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ContractSafeBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for ContractSafeBytes {
    fn from(value: Vec<u8>) -> Self {
        ContractSafeBytes(value)
    }
}

impl Display for ContractSafeBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.0.is_empty() {
            write!(f, "0x")?;
        }
        for byte in self.0.iter().take(MAX_DISPLAY_SIZE) {
            write!(f, "{byte:02X}")?;
        }
        // just some sanity safeguards
        if self.0.len() > MAX_DISPLAY_SIZE {
            write!(f, "...")?;
        }
        Ok(())
    }
}

// since cosmwasm stores everything with byte representation of stringified json, it's actually more efficient
// to serialize this as a string as opposed to keeping it as vector of bytes.
// for example vec![255,255] would have string representation of "[255,255]" and will be serialized to
// [91, 50, 53, 53, 44, 50, 53, 53, 93]. the equivalent base58 encoded string `"LUv"` will be serialized to
// [34, 76, 85, 118, 34]
//
// the difference between base58 and base64 is rather minimal and I've gone with base58 for consistency sake
impl Serialize for ContractSafeBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&bs58::encode(&self.0).into_string())
    }
}

impl<'de> Deserialize<'de> for ContractSafeBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        let bytes = bs58::decode(&s)
            .into_vec()
            .map_err(serde::de::Error::custom)?;
        Ok(ContractSafeBytes(bytes))
    }
}
