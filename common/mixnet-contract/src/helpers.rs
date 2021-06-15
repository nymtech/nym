// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crypto::asymmetric::{encryption, identity};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct IdentityStringPublicKeyWrapper(
    #[serde(with = "identity_public_key_string")]
    #[schemars(with = "String")]
    pub identity::PublicKey,
);

impl Deref for IdentityStringPublicKeyWrapper {
    type Target = identity::PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for IdentityStringPublicKeyWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct EncryptionStringPublicKeyWrapper(
    #[serde(with = "encryption_public_key_string")]
    #[schemars(with = "String")]
    pub encryption::PublicKey,
);

impl Deref for EncryptionStringPublicKeyWrapper {
    type Target = encryption::PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for EncryptionStringPublicKeyWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub mod identity_public_key_string {
    use crypto::asymmetric::identity;
    use serde::de::{Error as SerdeError, Unexpected, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt::{self, Formatter};

    pub fn serialize<S>(key: &identity::PublicKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&key.to_base58_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<identity::PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor;

        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = identity::PublicKey;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                write!(formatter, "Base58-encoded ed25519 public key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                identity::PublicKey::from_base58_string(v).map_err(|_| {
                    SerdeError::invalid_value(
                        Unexpected::Other("Properly formatted base58-encoded ed25519 public key"),
                        &self,
                    )
                })
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}

pub mod encryption_public_key_string {
    use crypto::asymmetric::encryption;
    use serde::de::{Error as SerdeError, Unexpected, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt::{self, Formatter};

    pub fn serialize<S>(key: &encryption::PublicKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&key.to_base58_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<encryption::PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor;

        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = encryption::PublicKey;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                write!(formatter, "Base58-encoded x25519 public key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                encryption::PublicKey::from_base58_string(v).map_err(|_| {
                    SerdeError::invalid_value(
                        Unexpected::Other("Properly formatted base58-encoded x25519 public key"),
                        &self,
                    )
                })
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}
