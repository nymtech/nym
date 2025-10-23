// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Key Derivation Functions using Blake3.

/// Derives a 32-byte key using Blake3's key derivation mode.
///
/// Uses Blake3's built-in `derive_key` function with domain separation via context string.
///
/// # Arguments
/// * `context` - Context string for domain separation (e.g., "nym-lp-psk-v1")
/// * `key_material` - Input key material (shared secret from ECDH, etc.)
/// * `salt` - Additional salt for freshness (timestamp + nonce)
///
/// # Returns
/// 32-byte derived key suitable for use as PSK
///
/// # Example
/// ```ignore
/// let psk = derive_key_blake3("nym-lp-psk-v1", shared_secret.as_bytes(), &salt);
/// ```
pub fn derive_key_blake3(context: &str, key_material: &[u8], salt: &[u8]) -> [u8; 32] {
    // Concatenate key_material and salt as input
    let input = [key_material, salt].concat();

    // Use Blake3's derive_key with context for domain separation
    // blake3::derive_key returns [u8; 32] directly
    blake3::derive_key(context, &input)
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

        assert_ne!(key1, key2, "Different contexts should produce different keys");
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

        assert_ne!(key1, key2, "Different key material should produce different keys");
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
