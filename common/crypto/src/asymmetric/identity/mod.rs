// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use ed25519_dalek::ed25519::signature::Signature as SignatureTrait;
pub use ed25519_dalek::SignatureError;
pub use ed25519_dalek::{Verifier, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH, SIGNATURE_LENGTH};
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "sphinx")]
use nym_sphinx_types::{DestinationAddressBytes, DESTINATION_ADDRESS_LENGTH};

#[cfg(feature = "rand")]
use rand::{CryptoRng, RngCore};
#[cfg(feature = "serde")]
use serde::de::Error as SerdeError;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "serde")]
use serde_bytes::{ByteBuf as SerdeByteBuf, Bytes as SerdeBytes};

#[derive(Debug, Error)]
pub enum Ed25519RecoveryError {
    #[error(transparent)]
    MalformedBytes(#[from] SignatureError),

    #[error("the base58 representation of the public key was malformed - {source}")]
    MalformedPublicKeyString {
        #[source]
        source: bs58::decode::Error,
    },

    #[error("the base58 representation of the private key was malformed - {source}")]
    MalformedPrivateKeyString {
        #[source]
        source: bs58::decode::Error,
    },

    #[error("the base58 representation of the signature was malformed - {source}")]
    MalformedSignatureString {
        #[source]
        source: bs58::decode::Error,
    },
}

/// Keypair for usage in ed25519 EdDSA.
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(crate = "serde_crate"))]
pub struct KeyPair {
    private_key: PrivateKey,

    // nothing secret about public key
    #[zeroize(skip)]
    public_key: PublicKey,
}

impl KeyPair {
    #[cfg(feature = "rand")]
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let ed25519_keypair = ed25519_dalek::Keypair::generate(rng);

        KeyPair {
            private_key: PrivateKey(ed25519_keypair.secret),
            public_key: PublicKey(ed25519_keypair.public),
        }
    }

    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, Ed25519RecoveryError> {
        Ok(KeyPair {
            private_key: PrivateKey::from_bytes(priv_bytes)?,
            public_key: PublicKey::from_bytes(pub_bytes)?,
        })
    }
}

impl PemStorableKeyPair for KeyPair {
    type PrivatePemKey = PrivateKey;
    type PublicPemKey = PublicKey;

    fn private_key(&self) -> &Self::PrivatePemKey {
        self.private_key()
    }
    fn public_key(&self) -> &Self::PublicPemKey {
        self.public_key()
    }

    fn from_keys(private_key: Self::PrivatePemKey, public_key: Self::PublicPemKey) -> Self {
        KeyPair {
            private_key,
            public_key,
        }
    }
}

/// ed25519 EdDSA Public Key
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PublicKey(ed25519_dalek::PublicKey);

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

impl PublicKey {
    #[cfg(feature = "sphinx")]
    pub fn derive_destination_address(&self) -> DestinationAddressBytes {
        let mut temporary_address = [0u8; DESTINATION_ADDRESS_LENGTH];
        let public_key_bytes = self.to_bytes();

        assert_eq!(DESTINATION_ADDRESS_LENGTH, PUBLIC_KEY_LENGTH);

        temporary_address.copy_from_slice(&public_key_bytes[..]);
        DestinationAddressBytes::from_bytes(temporary_address)
    }

    /// Convert this public key to a byte array.
    pub fn to_bytes(self) -> [u8; PUBLIC_KEY_LENGTH] {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, Ed25519RecoveryError> {
        Ok(PublicKey(ed25519_dalek::PublicKey::from_bytes(b)?))
    }

    pub fn to_base58_string(self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, Ed25519RecoveryError> {
        let bytes = bs58::decode(val)
            .into_vec()
            .map_err(|source| Ed25519RecoveryError::MalformedPublicKeyString { source })?;
        Self::from_bytes(&bytes)
    }

    pub fn verify<M: AsRef<[u8]>>(
        &self,
        message: M,
        signature: &Signature,
    ) -> Result<(), SignatureError> {
        self.0.verify(message.as_ref(), &signature.0)
    }
}

#[cfg(feature = "sphinx")]
impl From<PublicKey> for DestinationAddressBytes {
    fn from(value: PublicKey) -> Self {
        value.derive_destination_address()
    }
}

impl FromStr for PublicKey {
    type Err = Ed25519RecoveryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PublicKey::from_base58_string(s)
    }
}

#[cfg(feature = "serde")]
impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'d> Deserialize<'d> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        Ok(PublicKey(ed25519_dalek::PublicKey::deserialize(
            deserializer,
        )?))
    }
}

impl PemStorableKey for PublicKey {
    type Error = Ed25519RecoveryError;

    fn pem_type() -> &'static str {
        "ED25519 PUBLIC KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        (*self).to_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

/// ed25519 EdDSA Private Key
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct PrivateKey(ed25519_dalek::SecretKey);

impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

impl<'a> From<&'a PrivateKey> for PublicKey {
    fn from(pk: &'a PrivateKey) -> Self {
        PublicKey((&pk.0).into())
    }
}

impl PrivateKey {
    #[cfg(feature = "rand")]
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let ed25519_secret = ed25519_dalek::SecretKey::generate(rng);

        PrivateKey(ed25519_secret)
    }

    pub fn public_key(&self) -> PublicKey {
        self.into()
    }

    pub fn to_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, Ed25519RecoveryError> {
        Ok(PrivateKey(ed25519_dalek::SecretKey::from_bytes(b)?))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, Ed25519RecoveryError> {
        let bytes = bs58::decode(val)
            .into_vec()
            .map_err(|source| Ed25519RecoveryError::MalformedPrivateKeyString { source })?;
        Self::from_bytes(&bytes)
    }

    pub fn sign<M: AsRef<[u8]>>(&self, message: M) -> Signature {
        let expanded_secret_key = ed25519_dalek::ExpandedSecretKey::from(&self.0);
        let public_key: PublicKey = self.into();
        let sig = expanded_secret_key.sign(message.as_ref(), &public_key.0);
        Signature(sig)
    }

    /// Signs text with the provided Ed25519 private key, returning a base58 signature
    pub fn sign_text(&self, text: &str) -> String {
        let signature_bytes = self.sign(text).to_bytes();
        bs58::encode(signature_bytes).into_string()
    }
}

#[cfg(feature = "serde")]
impl Serialize for PrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'d> Deserialize<'d> for PrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        Ok(PrivateKey(ed25519_dalek::SecretKey::deserialize(
            deserializer,
        )?))
    }
}

impl PemStorableKey for PrivateKey {
    type Error = Ed25519RecoveryError;

    fn pem_type() -> &'static str {
        "ED25519 PRIVATE KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Signature(ed25519_dalek::Signature);

impl Signature {
    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, Ed25519RecoveryError> {
        let bytes = bs58::decode(val)
            .into_vec()
            .map_err(|source| Ed25519RecoveryError::MalformedSignatureString { source })?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> [u8; SIGNATURE_LENGTH] {
        self.0.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Ed25519RecoveryError> {
        Ok(Signature(ed25519_dalek::Signature::from_bytes(bytes)?))
    }
}

impl FromStr for Signature {
    type Err = Ed25519RecoveryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Signature::from_base58_string(s)
    }
}

#[cfg(feature = "serde")]
impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerdeBytes::new(&self.to_bytes()).serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'d> Deserialize<'d> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        let bytes = <SerdeByteBuf>::deserialize(deserializer)?;
        Signature::from_bytes(bytes.as_ref()).map_err(SerdeError::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    fn assert_zeroize<T: Zeroize>() {}

    #[test]
    fn private_key_is_zeroized() {
        assert_zeroize::<PrivateKey>();
        assert_zeroize_on_drop::<PrivateKey>();
    }
}
