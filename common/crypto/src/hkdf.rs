// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use hkdf::{
    hmac::{
        digest::{crypto_common::BlockSizeUser, Digest},
        SimpleHmac,
    },
    Hkdf,
};
use sha2::{Sha256, Sha512};

pub use hkdf::InvalidLength;
use zeroize::ZeroizeOnDrop;

/// Perform HKDF `extract` then `expand` as a single step.
pub fn extract_then_expand<D>(
    salt: Option<&[u8]>,
    ikm: &[u8],
    info: Option<&[u8]>,
    okm_length: usize,
) -> Result<Vec<u8>, hkdf::InvalidLength>
where
    D: Digest + BlockSizeUser + Clone,
{
    // TODO: this would need to change if we ever needed the generated pseudorandom key, but
    // realistically I don't see any reasons why we might need it

    let hkdf = Hkdf::<D, SimpleHmac<D>>::new(salt, ikm);
    let mut okm = vec![0u8; okm_length];
    hkdf.expand(info.unwrap_or(&[]), &mut okm)?;

    Ok(okm)
}

/// `DerivationMaterial` encapsulates parameters for deterministic key derivation using
/// HKDF (SHA-512).
///
/// It consists of:
///   - A master key (`master_key`): the base secret.
///   - An index (`index`): ensures unique derivations.
///   - A salt (`salt`): adds additional uniqueness, should be application specific.
///
/// Use the `derive_secret()` method to generate a 32-byte secret. To prepare for a new derivation,
/// call the `next()` method, which increments the index. **It is the caller's responsibility to
/// track and persist the derivation index if keys need to be rederived.**
///
/// # Example
///
/// ```rust
/// use nym_crypto::hkdf::DerivationMaterial;
///
/// let master_key = [0u8; 32]; // your secret master key
/// let salt = b"unique-salt-value";
/// let material = DerivationMaterial::new(master_key, 0, salt);
///
/// // Derive a secret
/// let secret = material.derive_secret().expect("Failed to derive secret");
///
/// // Prepare for the next derivation
/// let next_material = material.next();
/// ```
#[derive(ZeroizeOnDrop)]
pub struct DerivationMaterial {
    master_key: [u8; 32],
    index: u32,
    salt: Vec<u8>,
}

impl DerivationMaterial {
    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn salt(&self) -> &[u8] {
        &self.salt
    }

    /// Derives a 32-byte seed from a master seed and an index using HKDF (with SHA-512).
    ///
    /// The `salt` and the use of the index (as info) bind this derivation to an application/client.
    pub fn derive_secret(&self) -> Result<[u8; 32], hkdf::InvalidLength> {
        let salt = &self.salt;
        let info = self.index.to_be_bytes(); // Use the index as info
        let hk = Hkdf::<Sha512>::new(Some(salt), &self.master_key);
        let mut okm = [0u8; 32];
        hk.expand(&info, &mut okm)?;
        Ok(okm)
    }

    pub fn new<T: AsRef<[u8]>>(master_key: T, index: u32, salt: &[u8]) -> Self {
        // Coerce master_key to [u8; 32]
        let mut hasher = Sha256::new();
        hasher.update(master_key.as_ref());
        let master_key = hasher.finalize().into();

        Self {
            master_key,
            index,
            salt: salt.to_vec(),
        }
    }

    pub fn next(&self) -> Self {
        Self {
            master_key: self.master_key,
            index: self.index + 1,
            salt: self.salt.clone(),
        }
    }
}
