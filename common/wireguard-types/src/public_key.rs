// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use base64::engine::general_purpose;
use base64::Engine;
use serde::Serialize;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;

use x25519_dalek::PublicKey;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PeerPublicKey(PublicKey);

impl PeerPublicKey {
    #[allow(dead_code)]
    pub fn new(key: PublicKey) -> Self {
        PeerPublicKey(key)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for PeerPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(self.0.as_bytes()))
    }
}

impl Hash for PeerPublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state)
    }
}

impl Deref for PeerPublicKey {
    type Target = PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for PeerPublicKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key_bytes: Vec<u8> = general_purpose::STANDARD.decode(s).map_err(|source| {
            Error::MalformedPeerPublicKeyEncoding {
                pub_key: s.to_string(),
                source,
            }
        })?;

        let decoded_length = key_bytes.len();
        let Ok(key_arr): Result<[u8; 32], _> = key_bytes.try_into() else {
            return Err(Error::InvalidPeerPublicKeyLength {
                pub_key: s.to_string(),
                decoded_length,
            })?;
        };

        Ok(PeerPublicKey(PublicKey::from(key_arr)))
    }
}

impl Serialize for PeerPublicKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_key = general_purpose::STANDARD.encode(self.0.as_bytes());
        serializer.serialize_str(&encoded_key)
    }
}

impl<'de> serde::Deserialize<'de> for PeerPublicKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded_key = String::deserialize(deserializer)?;
        Ok(PeerPublicKey::from_str(&encoded_key).map_err(serde::de::Error::custom))?
    }
}
