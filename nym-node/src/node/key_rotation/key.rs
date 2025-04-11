// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::aes::cipher::crypto_common::rand_core::{CryptoRng, RngCore};
use nym_crypto::asymmetric::x25519;
use nym_pemstore::traits::PemStorableKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MalformedSphinxKey {
    #[error("inner x25519 key is malformed: {0}")]
    X25519Failure(#[from] x25519::KeyRecoveryError),

    #[error("did not receive sufficient number of bytes to recover the key")]
    Incomplete,
}

pub(crate) struct SphinxPrivateKey {
    rotation_id: u32,
    inner: x25519::PrivateKey,
}

impl SphinxPrivateKey {
    pub(crate) fn new<R: RngCore + CryptoRng>(rng: &mut R, rotation_id: u32) -> Self {
        SphinxPrivateKey {
            rotation_id,
            inner: x25519::PrivateKey::new(rng),
        }
    }

    pub(crate) fn x25519_pubkey(&self) -> x25519::PublicKey {
        self.inner.public_key()
    }
}

impl From<&SphinxPrivateKey> for SphinxPublicKey {
    fn from(value: &SphinxPrivateKey) -> Self {
        SphinxPublicKey {
            rotation_id: value.rotation_id,
            inner: (&value.inner).into(),
        }
    }
}

impl AsRef<x25519::PrivateKey> for SphinxPrivateKey {
    fn as_ref(&self) -> &x25519::PrivateKey {
        &self.inner
    }
}

// impl Deref for SphinxPrivateKey {
//     type Target = x25519::PrivateKey;
//
//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

pub(crate) struct SphinxPublicKey {
    pub(crate) rotation_id: u32,
    pub(crate) inner: x25519::PublicKey,
}

impl AsRef<x25519::PublicKey> for SphinxPublicKey {
    fn as_ref(&self) -> &x25519::PublicKey {
        &self.inner
    }
}

impl PemStorableKey for SphinxPrivateKey {
    type Error = MalformedSphinxKey;

    fn pem_type() -> &'static str {
        // it's fine (and actually desired) to attach 'SPHINX' here, as this is not a valid X25519 key by itself.
        // this is because it also contains the encoded rotation id
        "X25519 SPHINX PRIVATE KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.rotation_id
            .to_be_bytes()
            .into_iter()
            .chain(self.inner.to_bytes())
            .collect()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != x25519::PRIVATE_KEY_SIZE + 4 {
            return Err(MalformedSphinxKey::Incomplete);
        }
        // SAFETY: we just checked we have sufficient bytes available
        #[allow(clippy::unwrap_used)]
        let rotation_id = u32::from_be_bytes(bytes[..4].try_into().unwrap());

        Ok(SphinxPrivateKey {
            rotation_id,
            inner: x25519::PrivateKey::from_bytes(&bytes[4..])?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn private_key_bytes_convertion() {
        // Set up a deterministic RNG.
        let seed = [42u8; 32];
        let mut rng = ChaCha20Rng::from_seed(seed);

        let key = SphinxPrivateKey {
            rotation_id: 42,
            inner: x25519::PrivateKey::new(&mut rng),
        };

        let bytes = key.to_bytes();
        assert_eq!(bytes.len(), 36); // 32 bytes for x25519 key and 4 bytes for rotation id
        let recovered_key = SphinxPrivateKey::from_bytes(bytes.as_slice()).unwrap();

        assert_eq!(recovered_key.rotation_id, 42);
        assert_eq!(recovered_key.inner.to_bytes(), key.inner.to_bytes());
    }
}
