// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use hkdf::{
    Hkdf,
    hmac::{
        SimpleHmac,
        digest::{Digest, crypto_common::BlockSizeUser},
    },
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

pub mod blake3 {

    //! Key Derivation Functions using Blake3.

    use blake3::Hasher;

    use rand09::{RngCore, rng};
    use zeroize::Zeroize;

    pub fn derive_key_blake3_multi_input(
        info: &str,
        input_key_material: &[&[u8]],
        salt: &[u8],
    ) -> [u8; 32] {
        let mut hasher = Hasher::new_derive_key(info);

        for input_key in input_key_material {
            hasher.update(input_key);
        }

        hasher.update(salt);

        hasher.finalize().as_bytes().to_owned()
    }

    /// Derives a 32-byte key using Blake3's key derivation mode.
    ///
    /// Uses Blake3's built-in `derive_key` function with domain separation via context string.
    ///
    /// # Arguments
    /// * `info` - Context string for domain separation (e.g., "nym-lp-psk-v1")
    /// * `input_key_material` - Input key material (shared secret from ECDH, etc.)
    /// * `salt` - Additional salt for freshness (nonce)
    ///
    /// # Returns
    /// 32-byte derived key suitable for use as PSK
    ///
    /// # Example
    /// ```ignore
    /// let psk = derive_key_blake3("nym-lp-psk-v1", shared_secret.as_bytes(), &salt);
    /// ```
    pub fn derive_key_blake3(info: &str, input_key_material: &[u8], salt: &[u8]) -> [u8; 32] {
        derive_key_blake3_multi_input(info, &[input_key_material], salt)
    }

    pub fn derive_fresh_key_blake3_multi_input(
        info: &str,
        input_key_material: &[&[u8]],
    ) -> [u8; 32] {
        let mut salt = [0u8; 32];
        rng().fill_bytes(&mut salt);

        let derived_key = derive_key_blake3_multi_input(info, input_key_material, &salt);

        // Zeroize salt
        salt.zeroize();

        derived_key
    }

    /// Derives a fresh 32-byte key using Blake3's key derivation mode.
    /// The function calls a random number generator to generate a fresh salt.
    /// Uses Blake3's built-in `derive_key` function with domain separation via context string.
    ///
    /// # Arguments
    /// * `info` - Context string for domain separation (e.g., "nym-lp-psk-v1")
    /// * `input_key_material` - Input key material (shared secret from ECDH, etc.)
    ///
    /// # Returns
    /// 32-byte derived key suitable for use as PSK
    ///
    /// # Example
    /// ```ignore
    /// let psk = derive_fresh_key_blake3("nym-lp-psk-v1", shared_secret.as_bytes());
    /// ```
    pub fn derive_fresh_key_blake3(info: &str, input_key_material: &[u8]) -> [u8; 32] {
        derive_fresh_key_blake3_multi_input(info, &[input_key_material])
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_deterministic_derivation() {
            let context = "test-context";
            let key_material = b"shared_secret_12345";
            let salt = b"salt_67890";

            let key1 = derive_key_blake3(context, key_material, salt);
            let key2 = derive_key_blake3(context, key_material, salt);

            assert_eq!(key1, key2, "Same inputs should produce same output");
        }

        #[test]
        fn test_different_contexts_produce_different_keys() {
            let key_material = b"shared_secret";
            let salt = b"salt";

            let key1 = derive_key_blake3("context1", key_material, salt);
            let key2 = derive_key_blake3("context2", key_material, salt);

            assert_ne!(
                key1, key2,
                "Different contexts should produce different keys"
            );
        }

        #[test]
        fn test_different_salts_produce_different_keys() {
            let context = "test-context";
            let key_material = b"shared_secret";

            let key1 = derive_key_blake3(context, key_material, b"salt1");
            let key2 = derive_key_blake3(context, key_material, b"salt2");

            assert_ne!(key1, key2, "Different salts should produce different keys");
        }

        #[test]
        fn test_different_key_material_produces_different_keys() {
            let context = "test-context";
            let salt = b"salt";

            let key1 = derive_key_blake3(context, b"secret1", salt);
            let key2 = derive_key_blake3(context, b"secret2", salt);

            assert_ne!(
                key1, key2,
                "Different key material should produce different keys"
            );
        }

        #[test]
        fn test_output_length() {
            let key = derive_key_blake3("test", b"key", b"salt");
            assert_eq!(key.len(), 32, "Output should be exactly 32 bytes");
        }

        #[test]
        fn test_empty_inputs() {
            // Should not panic with empty inputs
            let key = derive_key_blake3("test", b"", b"");
            assert_eq!(key.len(), 32);
        }
    }
}
