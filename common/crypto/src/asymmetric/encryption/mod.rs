// Copyright 2020 Nym Technologies SA
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

use pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use rand::{rngs::OsRng, CryptoRng, RngCore};
use std::fmt::{self, Display, Formatter};

/// Size of a X25519 private key
pub const PRIVATE_KEY_SIZE: usize = 32;

/// Size of a X25519 public key
pub const PUBLIC_KEY_SIZE: usize = 32;

/// Size of a X25519 shared secret
pub const SHARED_SECRET_SIZE: usize = 32;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum EncryptionKeyError {
    InvalidPublicKey,
    InvalidPrivateKey,
}

// required for std::error::Error
impl Display for EncryptionKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionKeyError::InvalidPrivateKey => write!(f, "Invalid private key"),
            EncryptionKeyError::InvalidPublicKey => write!(f, "Invalid public key"),
        }
    }
}

impl std::error::Error for EncryptionKeyError {}

pub struct KeyPair {
    pub(crate) private_key: PrivateKey,
    pub(crate) public_key: PublicKey,
}

impl KeyPair {
    pub fn new() -> Self {
        let mut rng = OsRng;
        Self::new_with_rng(&mut rng)
    }

    pub fn new_with_rng<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let private_key = x25519_dalek::StaticSecret::new(rng);
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

    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, EncryptionKeyError> {
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

#[derive(Debug, Copy, Clone)]
pub struct PublicKey(x25519_dalek::PublicKey);

impl PublicKey {
    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_SIZE] {
        *self.0.as_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, EncryptionKeyError> {
        if b.len() != PUBLIC_KEY_SIZE {
            return Err(EncryptionKeyError::InvalidPublicKey);
        }
        let mut bytes = [0; PUBLIC_KEY_SIZE];
        bytes.copy_from_slice(&b[..PUBLIC_KEY_SIZE]);
        Ok(Self(x25519_dalek::PublicKey::from(bytes)))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<S: Into<String>>(val: S) -> Result<Self, EncryptionKeyError> {
        let bytes = bs58::decode(val.into())
            .into_vec()
            .expect("TODO: deal with this failure case");
        Self::from_bytes(&bytes)
    }
}

impl PemStorableKey for PublicKey {
    type Error = EncryptionKeyError;

    fn pem_type() -> &'static str {
        "X25519 PUBLIC KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

#[derive(Clone)]
pub struct PrivateKey(x25519_dalek::StaticSecret);

impl<'a> From<&'a PrivateKey> for PublicKey {
    fn from(pk: &'a PrivateKey) -> Self {
        PublicKey((&pk.0).into())
    }
}

impl PrivateKey {
    pub fn to_bytes(&self) -> [u8; PRIVATE_KEY_SIZE] {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, EncryptionKeyError> {
        if b.len() != PRIVATE_KEY_SIZE {
            return Err(EncryptionKeyError::InvalidPrivateKey);
        }
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&b[..PRIVATE_KEY_SIZE]);
        Ok(Self(x25519_dalek::StaticSecret::from(bytes)))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<S: Into<String>>(val: S) -> Result<Self, EncryptionKeyError> {
        let bytes = bs58::decode(val.into())
            .into_vec()
            .expect("TODO: deal with this failure case");
        Self::from_bytes(&bytes)
    }

    /// Perform a key exchange with another public key
    pub fn diffie_hellman(&self, remote_public: &PublicKey) -> [u8; SHARED_SECRET_SIZE] {
        *self.0.diffie_hellman(&remote_public.0).as_bytes()
    }
}

impl PemStorableKey for PrivateKey {
    type Error = EncryptionKeyError;

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

impl Into<nymsphinx_types::PublicKey> for PublicKey {
    fn into(self) -> nymsphinx_types::PublicKey {
        nymsphinx_types::PublicKey::from(self.to_bytes())
    }
}

impl<'a> Into<nymsphinx_types::PublicKey> for &'a PublicKey {
    fn into(self) -> nymsphinx_types::PublicKey {
        nymsphinx_types::PublicKey::from(self.to_bytes())
    }
}

impl From<nymsphinx_types::PublicKey> for PublicKey {
    fn from(pub_key: nymsphinx_types::PublicKey) -> Self {
        Self(x25519_dalek::PublicKey::from(*pub_key.as_bytes()))
    }
}

impl Into<nymsphinx_types::PrivateKey> for PrivateKey {
    fn into(self) -> nymsphinx_types::PrivateKey {
        nymsphinx_types::PrivateKey::from(self.to_bytes())
    }
}

impl<'a> Into<nymsphinx_types::PrivateKey> for &'a PrivateKey {
    fn into(self) -> nymsphinx_types::PrivateKey {
        nymsphinx_types::PrivateKey::from(self.to_bytes())
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
        for _ in 0..NUM_ITERATIONS {
            let keys = KeyPair::new();
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
