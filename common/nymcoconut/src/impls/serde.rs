use crate::elgamal::PrivateKey;
use crate::scheme::SecretKey;
use crate::{
    Base58, BlindSignRequest, BlindedSignature, PublicKey, Signature, VerificationKey,
    VerifyCredentialRequest,
};
use serde::de::Unexpected;
use serde::{de::Error, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

macro_rules! impl_serde {
    ($struct:ident, $visitor:ident) => {
        pub struct $visitor {}

        impl Serialize for $struct {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_bs58())
            }
        }

        impl<'de> Visitor<'de> for $visitor {
            type Value = $struct;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "A base58 encoded struct")
            }

            fn visit_str<E: Error>(self, s: &str) -> Result<Self::Value, E> {
                match $struct::try_from_bs58(s) {
                    Ok(x) => Ok(x),
                    Err(_) => Err(Error::invalid_value(Unexpected::Str(s), &self)),
                }
            }
        }

        impl<'de> Deserialize<'de> for $struct {
            fn deserialize<D>(deserializer: D) -> Result<$struct, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_str($visitor {})
            }
        }
    };
}

impl_serde!(SecretKey, V1);
impl_serde!(VerificationKey, V2);
impl_serde!(PublicKey, V3);
impl_serde!(PrivateKey, V4);
impl_serde!(BlindSignRequest, V5);
impl_serde!(BlindedSignature, V6);
impl_serde!(Signature, V7);
impl_serde!(VerifyCredentialRequest, V8);
