// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! PSK (Pre-Shared Key) derivation for LP sessions using Blake3 KDF.
//!
//! This module implements identity-bound PSK derivation where both client and gateway
//! derive the same PSK from their LP keypairs using ECDH + Blake3 KDF.

use crate::keypair::{PrivateKey, PublicKey};

/// Context string for Blake3 KDF domain separation.
const PSK_CONTEXT: &str = "nym-lp-psk-v1";

/// Derives a PSK using Blake3 KDF from local private key, remote public key, and salt.
///
/// # Formula
/// ```text
/// shared_secret = ECDH(local_private, remote_public)
/// psk = Blake3_derive_key(context="nym-lp-psk-v1", input=shared_secret || salt)
/// ```
///
/// # Properties
/// - **Identity-bound**: PSK is tied to the LP keypairs of both parties
/// - **Session-specific**: Different salts produce different PSKs
/// - **Symmetric**: Both sides derive the same PSK from their respective keys
///
/// # Arguments
/// * `local_private` - This side's LP private key
/// * `remote_public` - Peer's LP public key
/// * `salt` - 32-byte salt (timestamp + nonce from ClientHello)
///
/// # Returns
/// 32-byte PSK suitable for Noise protocol
///
/// # Example
/// ```ignore
/// // Client side
/// let client_private = client_keypair.private_key();
/// let gateway_public = gateway_keypair.public_key();
/// let salt = ClientHelloData::new_with_fresh_salt(...).salt;
/// let psk = derive_psk(&client_private, &gateway_public, &salt);
///
/// // Gateway side (derives same PSK)
/// let gateway_private = gateway_keypair.private_key();
/// let client_public = /* from ClientHello */;
/// let psk = derive_psk(&gateway_private, &client_public, &salt);
/// ```
pub fn derive_psk(
    local_private: &PrivateKey,
    remote_public: &PublicKey,
    salt: &[u8; 32],
) -> [u8; 32] {
    // Perform ECDH to get shared secret
    let shared_secret = local_private.diffie_hellman(remote_public);

    // Derive PSK using Blake3 KDF with domain separation
    nym_crypto::kdf::derive_key_blake3(PSK_CONTEXT, shared_secret.as_bytes(), salt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::Keypair;

    #[test]
    fn test_psk_derivation_is_deterministic() {
        let keypair_1 = Keypair::default();
        let keypair_2 = Keypair::default();
        let salt = [1u8; 32];

        // Derive PSK twice with same inputs
        let psk1 = derive_psk(
            keypair_1.private_key(),
            keypair_2.public_key(),
            &salt,
        );
        let psk2 = derive_psk(
            keypair_1.private_key(),
            keypair_2.public_key(),
            &salt,
        );

        assert_eq!(psk1, psk2, "Same inputs should produce same PSK");
    }

    #[test]
    fn test_psk_derivation_is_symmetric() {
        let keypair_1 = Keypair::default();
        let keypair_2 = Keypair::default();
        let salt = [2u8; 32];

        // Client derives PSK
        let client_psk = derive_psk(
            keypair_1.private_key(),
            keypair_2.public_key(),
            &salt,
        );

        // Gateway derives PSK from their perspective
        let gateway_psk = derive_psk(
            keypair_2.private_key(),
            keypair_1.public_key(),
            &salt,
        );

        assert_eq!(
            client_psk, gateway_psk,
            "Both sides should derive identical PSK"
        );
    }

    #[test]
    fn test_different_salts_produce_different_psks() {
        let keypair_1 = Keypair::default();
        let keypair_2 = Keypair::default();

        let salt1 = [1u8; 32];
        let salt2 = [2u8; 32];

        let psk1 = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt1);
        let psk2 = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt2);

        assert_ne!(psk1, psk2, "Different salts should produce different PSKs");
    }

    #[test]
    fn test_different_keys_produce_different_psks() {
        let keypair_1 = Keypair::default();
        let keypair_2 = Keypair::default();
        let keypair_3 = Keypair::default();
        let salt = [3u8; 32];

        let psk1 = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt);
        let psk2 = derive_psk(keypair_1.private_key(), keypair_3.public_key(), &salt);

        assert_ne!(
            psk1, psk2,
            "Different remote keys should produce different PSKs"
        );
    }

    #[test]
    fn test_psk_output_length() {
        let keypair_1 = Keypair::default();
        let keypair_2 = Keypair::default();
        let salt = [4u8; 32];

        let psk = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt);

        assert_eq!(psk.len(), 32, "PSK should be exactly 32 bytes");
    }
}
