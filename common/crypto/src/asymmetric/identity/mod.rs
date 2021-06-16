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

#[cfg(not(feature = "minimum-asymmetric"))]
use ed25519_dalek::ed25519::signature::Signature as SignatureTrait;
pub use ed25519_dalek::SignatureError;
#[cfg(not(feature = "minimum-asymmetric"))]
pub use ed25519_dalek::Verifier;
pub use ed25519_dalek::{PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH, SIGNATURE_LENGTH};
#[cfg(not(feature = "minimum-asymmetric"))]
use nymsphinx_types::{DestinationAddressBytes, DESTINATION_ADDRESS_LENGTH};
#[cfg(not(feature = "minimum-asymmetric"))]
use pemstore::traits::{PemStorableKey, PemStorableKeyPair};
#[cfg(not(feature = "minimum-asymmetric"))]
use rand::{CryptoRng, RngCore};
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum KeyRecoveryError {
    #[cfg(not(feature = "minimum-asymmetric"))]
    MalformedBytes(SignatureError),
    MalformedString(bs58::decode::Error),
}
#[cfg(not(feature = "minimum-asymmetric"))]
impl From<SignatureError> for KeyRecoveryError {
    fn from(err: SignatureError) -> Self {
        KeyRecoveryError::MalformedBytes(err)
    }
}

impl From<bs58::decode::Error> for KeyRecoveryError {
    fn from(err: bs58::decode::Error) -> Self {
        KeyRecoveryError::MalformedString(err)
    }
}

impl fmt::Display for KeyRecoveryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(not(feature = "minimum-asymmetric"))]
            KeyRecoveryError::MalformedBytes(err) => write!(f, "malformed bytes - {}", err),
            KeyRecoveryError::MalformedString(err) => write!(f, "malformed string - {}", err),
        }
    }
}

impl std::error::Error for KeyRecoveryError {}

#[cfg(not(feature = "minimum-asymmetric"))]
/// Keypair for usage in ed25519 EdDSA.
pub struct KeyPair {
    private_key: PrivateKey,
    public_key: PublicKey,
}

#[cfg(not(feature = "minimum-asymmetric"))]
impl KeyPair {
    #[cfg(not(feature = "minimum-asymmetric"))]
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

    #[cfg(not(feature = "minimum-asymmetric"))]
    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, KeyRecoveryError> {
        Ok(KeyPair {
            private_key: PrivateKey::from_bytes(priv_bytes)?,
            public_key: PublicKey::from_bytes(pub_bytes)?,
        })
    }
}

#[cfg(not(feature = "minimum-asymmetric"))]
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
#[cfg_attr(
    feature = "serde-serialize",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PublicKey(ed25519_dalek::PublicKey);

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PublicKey {
    #[cfg(not(feature = "minimum-asymmetric"))]
    pub fn derive_destination_address(&self) -> DestinationAddressBytes {
        let mut temporary_address = [0u8; DESTINATION_ADDRESS_LENGTH];
        let public_key_bytes = self.to_bytes();

        assert_eq!(DESTINATION_ADDRESS_LENGTH, PUBLIC_KEY_LENGTH);

        temporary_address.copy_from_slice(&public_key_bytes[..]);
        DestinationAddressBytes::from_bytes(temporary_address)
    }

    /// Convert this public key to a byte array.
    #[inline]
    pub fn to_bytes(self) -> [u8; PUBLIC_KEY_LENGTH] {
        self.0.to_bytes()
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn from_bytes(b: &[u8]) -> Result<Self, KeyRecoveryError> {
        todo!()
        // Ok(PublicKey(ed25519_dalek::PublicKey::from_bytes(b)?))
    }

    pub fn to_base58_string(self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, KeyRecoveryError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    #[cfg(not(feature = "minimum-asymmetric"))]
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), SignatureError> {
        self.0.verify(message, &signature.0)
    }
}

#[cfg(not(feature = "minimum-asymmetric"))]
impl PemStorableKey for PublicKey {
    type Error = KeyRecoveryError;

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

#[cfg(not(feature = "minimum-asymmetric"))]
/// ed25519 EdDSA Private Key\
#[derive(Debug)]
pub struct PrivateKey(ed25519_dalek::SecretKey);

#[cfg(not(feature = "minimum-asymmetric"))]
impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

#[cfg(not(feature = "minimum-asymmetric"))]
impl<'a> From<&'a PrivateKey> for PublicKey {
    fn from(pk: &'a PrivateKey) -> Self {
        PublicKey((&pk.0).into())
    }
}

#[cfg(not(feature = "minimum-asymmetric"))]
impl PrivateKey {
    pub fn to_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, KeyRecoveryError> {
        Ok(PrivateKey(ed25519_dalek::SecretKey::from_bytes(b)?))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, KeyRecoveryError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    #[cfg(not(feature = "minimum-asymmetric"))]
    pub fn sign(&self, message: &[u8]) -> Signature {
        let expanded_secret_key = ed25519_dalek::ExpandedSecretKey::from(&self.0);
        let public_key: PublicKey = self.into();
        let sig = expanded_secret_key.sign(message, &public_key.0);
        Signature(sig)
    }
}

#[cfg(not(feature = "minimum-asymmetric"))]
impl PemStorableKey for PrivateKey {
    type Error = KeyRecoveryError;

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

#[cfg(not(feature = "minimum-asymmetric"))]
#[derive(Debug)]
pub struct Signature(ed25519_dalek::Signature);

#[cfg(not(feature = "minimum-asymmetric"))]
impl Signature {
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LENGTH] {
        self.0.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SignatureError> {
        Ok(Signature(ed25519_dalek::Signature::from_bytes(bytes)?))
    }
}
