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

use crate::{PemStorableKey, PemStorableKeyPair};
use bs58;
use ed25519_dalek::{SignatureError, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use nymsphinx::DestinationAddressBytes;
use rand::{rngs::OsRng, CryptoRng, RngCore};

/// Keypair for usage in ed25519 EdDSA.
pub struct MixIdentityKeyPair {
    private_key: MixIdentityPrivateKey,
    public_key: MixIdentityPublicKey,
}

impl MixIdentityKeyPair {
    pub fn new() -> Self {
        let mut rng = OsRng;
        Self::new_with_rng(&mut rng)
    }

    pub fn new_with_rng<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let ed25519_keypair = ed25519_dalek::Keypair::generate(rng);

        MixIdentityKeyPair {
            private_key: MixIdentityPrivateKey(ed25519_keypair.secret),
            public_key: MixIdentityPublicKey(ed25519_keypair.public),
        }
    }

    pub fn private_key(&self) -> &MixIdentityPrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &MixIdentityPublicKey {
        &self.public_key
    }

    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, SignatureError> {
        Ok(MixIdentityKeyPair {
            private_key: MixIdentityPrivateKey::from_bytes(priv_bytes)?,
            public_key: MixIdentityPublicKey::from_bytes(pub_bytes)?,
        })
    }
}

impl PemStorableKeyPair for MixIdentityKeyPair {
    type PrivatePemKey = MixIdentityPrivateKey;
    type PublicPemKey = MixIdentityPublicKey;
    type Error = SignatureError;

    fn private_key(&self) -> &Self::PrivatePemKey {
        self.private_key()
    }

    fn public_key(&self) -> &Self::PublicPemKey {
        self.public_key()
    }

    fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, SignatureError> {
        Self::from_bytes(priv_bytes, pub_bytes)
    }
}

/// ed25519 EdDSA Public Key
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MixIdentityPublicKey(ed25519_dalek::PublicKey);

impl MixIdentityPublicKey {
    pub fn derive_address(&self) -> DestinationAddressBytes {
        let mut temporary_address = [0u8; 32];
        let public_key_bytes = self.to_bytes();
        temporary_address.copy_from_slice(&public_key_bytes[..]);

        DestinationAddressBytes::from_bytes(temporary_address)
    }

    /// Convert this public key to a byte array.
    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_LENGTH] {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, SignatureError> {
        Ok(MixIdentityPublicKey(ed25519_dalek::PublicKey::from_bytes(
            b,
        )?))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<S: Into<String>>(val: S) -> Result<Self, SignatureError> {
        let bytes = bs58::decode(val.into())
            .into_vec()
            .expect("TODO: deal with this failure case");
        Self::from_bytes(&bytes)
    }
}

impl PemStorableKey for MixIdentityPublicKey {
    fn pem_type(&self) -> String {
        String::from("ED25519 PUBLIC KEY")
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/// ed25519 EdDSA Private Key
#[derive(Debug)]
pub struct MixIdentityPrivateKey(ed25519_dalek::SecretKey);

impl<'a> From<&'a MixIdentityPrivateKey> for MixIdentityPublicKey {
    fn from(pk: &'a MixIdentityPrivateKey) -> Self {
        let public = ed25519_dalek::PublicKey::from(&pk.0);
        MixIdentityPublicKey(public)
    }
}

impl MixIdentityPrivateKey {
    pub fn to_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, SignatureError> {
        Ok(MixIdentityPrivateKey(ed25519_dalek::SecretKey::from_bytes(
            b,
        )?))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<S: Into<String>>(val: S) -> Result<Self, SignatureError> {
        let bytes = bs58::decode(val.into())
            .into_vec()
            .expect("TODO: deal with this failure case");
        Self::from_bytes(&bytes)
    }
}

impl PemStorableKey for MixIdentityPrivateKey {
    fn pem_type(&self) -> String {
        String::from("ED25519 PRIVATE KEY")
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}
