// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use pemstore::traits::{PemStorableKey, PemStorableKeyPair};
#[cfg(feature = "rand")]
use rand::{CryptoRng, RngCore};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display, Formatter};

/// Size of a X25519 private key
pub const PRIVATE_KEY_SIZE: usize = 32;

/// Size of a X25519 public key
pub const PUBLIC_KEY_SIZE: usize = 32;

/// Size of a X25519 shared secret
pub const SHARED_SECRET_SIZE: usize = 32;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum KeyRecoveryError {
    InvalidPublicKeyBytes,
    InvalidPrivateKeyBytes,
    MalformedString(bs58::decode::Error),
}

impl From<bs58::decode::Error> for KeyRecoveryError {
    fn from(err: bs58::decode::Error) -> Self {
        KeyRecoveryError::MalformedString(err)
    }
}

// required for std::error::Error
impl Display for KeyRecoveryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            KeyRecoveryError::InvalidPrivateKeyBytes => write!(f, "Invalid private key bytes"),
            KeyRecoveryError::InvalidPublicKeyBytes => write!(f, "Invalid public key bytes"),
            KeyRecoveryError::MalformedString(err) => write!(f, "malformed string - {}", err),
        }
    }
}

impl std::error::Error for KeyRecoveryError {}

pub struct KeyPair {
    pub(crate) private_key: PrivateKey,
    pub(crate) public_key: PublicKey,
}

impl KeyPair {
    #[cfg(feature = "rand")]
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let private_key = x25519_dalek::StaticSecret::new(rng);
        // false positive on nightly clippy (1.64.0)
        #[allow(clippy::needless_borrow)]
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

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct PublicKey(x25519_dalek::PublicKey);

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

impl PublicKey {
    pub fn to_bytes(self) -> [u8; PUBLIC_KEY_SIZE] {
        *self.0.as_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, KeyRecoveryError> {
        if b.len() != PUBLIC_KEY_SIZE {
            return Err(KeyRecoveryError::InvalidPublicKeyBytes);
        }
        let mut bytes = [0; PUBLIC_KEY_SIZE];
        bytes.copy_from_slice(&b[..PUBLIC_KEY_SIZE]);
        Ok(Self(x25519_dalek::PublicKey::from(bytes)))
    }

    pub fn to_base58_string(self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, KeyRecoveryError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
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

pub struct PrivateKey(x25519_dalek::StaticSecret);

impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

impl<'a> From<&'a PrivateKey> for PublicKey {
    fn from(pk: &'a PrivateKey) -> Self {
        // false positive on nightly clippy (1.64.0)
        #[allow(clippy::needless_borrow)]
        PublicKey((&pk.0).into())
    }
}

impl PrivateKey {
    pub fn to_bytes(&self) -> [u8; PRIVATE_KEY_SIZE] {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, KeyRecoveryError> {
        if b.len() != PRIVATE_KEY_SIZE {
            return Err(KeyRecoveryError::InvalidPrivateKeyBytes);
        }
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&b[..PRIVATE_KEY_SIZE]);
        Ok(Self(x25519_dalek::StaticSecret::from(bytes)))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, KeyRecoveryError> {
        let bytes = bs58::decode(val).into_vec()?;
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

// compatibility with sphinx keys:
impl From<PublicKey> for nymsphinx_types::PublicKey {
    fn from(key: PublicKey) -> Self {
        nymsphinx_types::PublicKey::from(key.to_bytes())
    }
}

impl<'a> From<&'a PublicKey> for nymsphinx_types::PublicKey {
    fn from(key: &'a PublicKey) -> Self {
        nymsphinx_types::PublicKey::from((*key).to_bytes())
    }
}

impl From<nymsphinx_types::PublicKey> for PublicKey {
    fn from(pub_key: nymsphinx_types::PublicKey) -> Self {
        Self(x25519_dalek::PublicKey::from(*pub_key.as_bytes()))
    }
}

impl From<PrivateKey> for nymsphinx_types::PrivateKey {
    fn from(key: PrivateKey) -> Self {
        nymsphinx_types::PrivateKey::from(key.to_bytes())
    }
}

impl<'a> From<&'a PrivateKey> for nymsphinx_types::PrivateKey {
    fn from(key: &'a PrivateKey) -> Self {
        nymsphinx_types::PrivateKey::from(key.to_bytes())
    }
}

impl From<nymsphinx_types::PrivateKey> for PrivateKey {
    fn from(private_key: nymsphinx_types::PrivateKey) -> Self {
        let private_key_bytes = private_key.to_bytes();
        assert_eq!(private_key_bytes.len(), PRIVATE_KEY_SIZE);
        Self::from_bytes(&private_key_bytes).unwrap()
    }
}

#[cfg(test)]
mod sphinx_key_conversion {
    use super::*;

    const NUM_ITERATIONS: usize = 100;

    #[test]
    fn works_for_forward_conversion() {
        let mut rng = rand::rngs::OsRng;

        for _ in 0..NUM_ITERATIONS {
            let keys = KeyPair::new(&mut rng);
            let private = keys.private_key;
            let public = keys.public_key;

            let private_bytes = private.to_bytes();
            let public_bytes = public.to_bytes();

            let sphinx_private: nymsphinx_types::PrivateKey = private.into();
            let recovered_private = PrivateKey::from(sphinx_private);

            let sphinx_public: nymsphinx_types::PublicKey = public.into();
            let recovered_public = PublicKey::from(sphinx_public);
            assert_eq!(private_bytes, recovered_private.to_bytes());
            assert_eq!(public_bytes, recovered_public.to_bytes());
        }
    }

    #[test]
    fn works_for_backward_conversion() {
        for _ in 0..NUM_ITERATIONS {
            let (sphinx_private, sphinx_public) = nymsphinx_types::crypto::keygen();

            let private_bytes = sphinx_private.to_bytes();
            let public_bytes = sphinx_public.as_bytes();

            let private: PrivateKey = sphinx_private.into();
            let recovered_sphinx_private: nymsphinx_types::PrivateKey = private.into();

            let public: PublicKey = sphinx_public.into();
            let recovered_sphinx_public: nymsphinx_types::PublicKey = public.into();
            assert_eq!(private_bytes, recovered_sphinx_private.to_bytes());
            assert_eq!(public_bytes, recovered_sphinx_public.as_bytes());
        }
    }
}
