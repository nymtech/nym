// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use hkdf::{
    hmac::{
        digest::{crypto_common::BlockSizeUser, Digest},
        SimpleHmac,
    },
    Hkdf,
};
use sha2::Sha512;

pub use hkdf::InvalidLength;

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

pub struct DerivationMaterial {
    master_key: [u8; 32],
    index: u32,
    salt: String,
}

impl DerivationMaterial {
    /// Derives a 32-byte seed from a master seed and an index using HKDF (with SHA-512).
    ///
    /// The `salt` and the use of the index (as info) bind this derivation to an application/client.
    pub fn derive_secret(&self) -> Result<[u8; 32], hkdf::InvalidLength> {
        let salt = self.salt.as_bytes();
        let info = self.index.to_be_bytes(); // Use the index as info
        let hk = Hkdf::<Sha512>::new(Some(salt), &self.master_key);
        let mut okm = [0u8; 32];
        hk.expand(&info, &mut okm)?;
        Ok(okm)
    }

    pub fn new(master_key: [u8; 32], index: u32, salt: String) -> Self {
        Self {
            master_key,
            index,
            salt,
        }
    }
}
