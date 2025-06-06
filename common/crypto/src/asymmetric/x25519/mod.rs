// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "rand")]
use rand::{CryptoRng, RngCore};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "serde")]
pub mod serde_helpers;

/// Size of a X25519 private key
pub const PRIVATE_KEY_SIZE: usize = 32;

/// Size of a X25519 public key
pub const PUBLIC_KEY_SIZE: usize = 32;

/// Size of a X25519 shared secret
pub const SHARED_SECRET_SIZE: usize = 32;

#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum KeyRecoveryError {
    #[error("received public key of invalid size. Got: {received}, expected: {expected}")]
    InvalidSizePublicKey { received: usize, expected: usize },

    #[error("received private key of invalid size. Got: {received}, expected: {expected}")]
    InvalidSizePrivateKey { received: usize, expected: usize },

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
}

#[derive(Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct KeyPair {
    pub(crate) private_key: PrivateKey,

    // nothing secret about public key
    #[zeroize(skip)]
    pub(crate) public_key: PublicKey,
}

impl KeyPair {
    #[cfg(feature = "rand")]
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let private_key = x25519_dalek::StaticSecret::random_from_rng(rng);
        let public_key = (&private_key).into();

        KeyPair {
            private_key: PrivateKey(private_key),
            public_key: PublicKey(public_key),
        }
    }

    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, KeyRecoveryError> {
        Ok(KeyPair {
            private_key: PrivateKey::from_bytes(priv_bytes)?,
            public_key: PublicKey::from_bytes(pub_bytes)?,
        })
    }
}

impl From<PrivateKey> for KeyPair {
    fn from(private_key: PrivateKey) -> Self {
        KeyPair {
            public_key: (&private_key).into(),
            private_key,
        }
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

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub struct PublicKey(x25519_dalek::PublicKey);

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_base58_string(), f)
    }
}

impl Debug for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.to_base58_string(), f)
    }
}

impl PublicKey {
    pub fn to_bytes(self) -> [u8; PUBLIC_KEY_SIZE] {
        *self.0.as_bytes()
    }

    pub fn as_bytes(&self) -> &[u8; PUBLIC_KEY_SIZE] {
        self.0.as_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, KeyRecoveryError> {
        if b.len() != PUBLIC_KEY_SIZE {
            return Err(KeyRecoveryError::InvalidSizePublicKey {
                received: b.len(),
                expected: PUBLIC_KEY_SIZE,
            });
        }
        let mut bytes = [0; PUBLIC_KEY_SIZE];
        bytes.copy_from_slice(&b[..PUBLIC_KEY_SIZE]);
        Ok(Self(x25519_dalek::PublicKey::from(bytes)))
    }

    pub fn to_base58_string(self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, KeyRecoveryError> {
        let bytes = bs58::decode(val)
            .into_vec()
            .map_err(|source| KeyRecoveryError::MalformedPublicKeyString { source })?;
        Self::from_bytes(&bytes)
    }
}

impl FromStr for PublicKey {
    type Err = KeyRecoveryError;

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
        Ok(PublicKey(x25519_dalek::PublicKey::deserialize(
            deserializer,
        )?))
    }
}

impl PemStorableKey for PublicKey {
    type Error = KeyRecoveryError;

    fn pem_type() -> &'static str {
        "X25519 PUBLIC KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        (*self).to_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl From<x25519_dalek::PublicKey> for PublicKey {
    fn from(public_key: x25519_dalek::PublicKey) -> Self {
        PublicKey(public_key)
    }
}

impl From<PublicKey> for x25519_dalek::PublicKey {
    fn from(public_key: PublicKey) -> Self {
        public_key.0
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PrivateKey(x25519_dalek::StaticSecret);

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
impl FromStr for PrivateKey {
    type Err = KeyRecoveryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PrivateKey::from_base58_string(s)
    }
}

impl PrivateKey {
    #[cfg(feature = "rand")]
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let x25519_secret = x25519_dalek::StaticSecret::random_from_rng(rng);

        PrivateKey(x25519_secret)
    }

    pub fn inner(&self) -> &x25519_dalek::StaticSecret {
        &self.0
    }

    pub fn public_key(&self) -> PublicKey {
        self.into()
    }

    pub fn to_bytes(&self) -> [u8; PRIVATE_KEY_SIZE] {
        self.0.to_bytes()
    }

    pub fn as_bytes(&self) -> &[u8; PRIVATE_KEY_SIZE] {
        self.0.as_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, KeyRecoveryError> {
        if b.len() != PRIVATE_KEY_SIZE {
            return Err(KeyRecoveryError::InvalidSizePrivateKey {
                received: b.len(),
                expected: PRIVATE_KEY_SIZE,
            });
        }
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&b[..PRIVATE_KEY_SIZE]);
        Ok(Self(x25519_dalek::StaticSecret::from(bytes)))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, KeyRecoveryError> {
        let bytes = bs58::decode(val)
            .into_vec()
            .map_err(|source| KeyRecoveryError::MalformedPrivateKeyString { source })?;
        Self::from_bytes(&bytes)
    }

    /// Perform a key exchange with another public key
    pub fn diffie_hellman(&self, remote_public: &PublicKey) -> [u8; SHARED_SECRET_SIZE] {
        *self.0.diffie_hellman(&remote_public.0).as_bytes()
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
        Ok(PrivateKey(x25519_dalek::StaticSecret::deserialize(
            deserializer,
        )?))
    }
}

impl PemStorableKey for PrivateKey {
    type Error = KeyRecoveryError;

    fn pem_type() -> &'static str {
        "X25519 PRIVATE KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl From<x25519_dalek::StaticSecret> for PrivateKey {
    fn from(secret: x25519_dalek::StaticSecret) -> Self {
        PrivateKey(secret)
    }
}

impl AsRef<x25519_dalek::StaticSecret> for PrivateKey {
    fn as_ref(&self) -> &x25519_dalek::StaticSecret {
        &self.0
    }
}

impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
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
