// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use hkdf::hmac::digest::{Digest, crypto_common::BlockSizeUser};

pub use hkdf::{InvalidLength, *};

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

    let hkdf = SimpleHkdf::<D>::new(salt, ikm);
    let mut okm = vec![0u8; okm_length];
    hkdf.expand(info.unwrap_or(&[]), &mut okm)?;

    Ok(okm)
}

pub mod blake3 {

    //! Key Derivation Functions using Blake3.

    use blake3::Hasher;

    use rand::{Rng, rng};
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
