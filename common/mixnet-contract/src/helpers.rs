use crypto::asymmetric::{encryption, identity};
use serde::de::{Error as SerdeError, Unexpected, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;
use std::fmt::Formatter;

pub(crate) fn se_identity_public_key_string<S>(
    key: &identity::PublicKey,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&key.to_base58_string())
}

pub(crate) fn de_identity_public_key_string<'de, D>(
    deserializer: D,
) -> Result<identity::PublicKey, D::Error>
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
            identity::PublicKey::from_base58_string(v).map_err(|err| {
                SerdeError::invalid_value(
                    Unexpected::Other("Properly formatted base58-encoded ed25519 public key"),
                    &self,
                )
            })
        }
    }

    deserializer.deserialize_str(KeyVisitor)
}

pub(crate) fn se_encryption_public_key_string<S>(
    key: &encryption::PublicKey,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&key.to_base58_string())
}

pub(crate) fn de_encryption_public_key_string<'de, D>(
    deserializer: D,
) -> Result<encryption::PublicKey, D::Error>
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
            encryption::PublicKey::from_base58_string(v).map_err(|err| {
                SerdeError::invalid_value(
                    Unexpected::Other("Properly formatted base58-encoded x25519 public key"),
                    &self,
                )
            })
        }
    }

    deserializer.deserialize_str(KeyVisitor)
}
