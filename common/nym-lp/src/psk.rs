// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! PSK (Pre-Shared Key) derivation for LP sessions using Blake3 KDF.
//!
//! This module implements identity-bound PSK derivation where both client and gateway
//! derive the same PSK from their LP keypairs.
//!
//! Two approaches are supported:
//! - **Legacy ECDH-only** (`derive_psk`) - Simple but no post-quantum security
//! - **PSQ-enhanced** (`derive_psk_with_psq_*`) - Combines ECDH with post-quantum KEM

use crate::keypair::{PrivateKey, PublicKey};
use crate::LpError;
use libcrux_psq::v1::cred::{Authenticator, Ed25519};
use libcrux_psq::v1::impls::X25519 as PsqX25519;
use libcrux_psq::v1::psk_registration::{Initiator, InitiatorMsg, Responder};
use libcrux_psq::v1::traits::{Ciphertext as PsqCiphertext, PSQ};
use nym_crypto::asymmetric::ed25519;
use nym_kkt::ciphersuite::{DecapsulationKey, EncapsulationKey};
use std::time::Duration;
use tls_codec::{Deserialize as TlsDeserializeTrait, Serialize as TlsSerializeTrait};

/// Context string for Blake3 KDF domain separation (legacy).
const PSK_CONTEXT: &str = "nym-lp-psk-v1";

/// Context string for Blake3 KDF domain separation (PSQ-enhanced).
const PSK_PSQ_CONTEXT: &str = "nym-lp-psk-psq-v1";

/// Session context for PSQ protocol.
const PSQ_SESSION_CONTEXT: &[u8] = b"nym-lp-psq-session";

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
///
/// # Deprecation Warning
/// This function provides no post-quantum security. Use `derive_psk_with_psq_initiator`
/// or `derive_psk_with_psq_responder` for HNDL (Harvest-Now, Decrypt-Later) resistance.
#[deprecated(
    since = "0.1.0",
    note = "Use derive_psk_with_psq_* for post-quantum security"
)]
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

/// Derives a PSK using PSQ (Post-Quantum Secure PSK) protocol - Initiator side.
///
/// This function combines classical ECDH with post-quantum KEM to provide forward secrecy
/// and HNDL (Harvest-Now, Decrypt-Later) resistance.
///
/// # Formula
/// ```text
/// ecdh_secret = ECDH(local_x25519_private, remote_x25519_public)
/// (psq_psk, ct) = PSQ_Encapsulate(remote_kem_public, session_context)
/// psk = Blake3_derive_key(
///     context="nym-lp-psk-psq-v1",
///     input=ecdh_secret || psq_psk || salt
/// )
/// ```
///
/// # Arguments
/// * `local_x25519_private` - Initiator's X25519 private key (for Noise)
/// * `remote_x25519_public` - Responder's X25519 public key (for Noise)
/// * `remote_kem_public` - Responder's KEM public key (obtained via KKT)
/// * `salt` - 32-byte salt for session binding
///
/// # Returns
/// * `Ok((psk, ciphertext))` - PSK and ciphertext to send to responder
/// * `Err(LpError)` - If PSQ encapsulation fails
///
/// # Example
/// ```ignore
/// // Client side (after KKT exchange)
/// let (psk, ciphertext) = derive_psk_with_psq_initiator(
///     client_x25519_private,
///     gateway_x25519_public,
///     &gateway_kem_key,  // from KKT
///     &salt
/// )?;
/// // Send ciphertext to gateway
/// ```
pub fn derive_psk_with_psq_initiator(
    local_x25519_private: &PrivateKey,
    remote_x25519_public: &PublicKey,
    remote_kem_public: &EncapsulationKey,
    salt: &[u8; 32],
) -> Result<([u8; 32], Vec<u8>), LpError> {
    // Step 1: Classical ECDH for baseline security
    let ecdh_secret = local_x25519_private.diffie_hellman(remote_x25519_public);

    // Step 2: PSQ encapsulation for post-quantum security
    // Extract X25519 public key from EncapsulationKey
    let kem_pk = match remote_kem_public {
        EncapsulationKey::X25519(pk) => pk,
        _ => {
            return Err(LpError::KKTError(
                "Only X25519 KEM is currently supported for PSQ".to_string(),
            ))
        }
    };

    let mut rng = rand09::rng();
    let (psq_psk, ciphertext) = PsqX25519::encapsulate_psq(&kem_pk, PSQ_SESSION_CONTEXT, &mut rng)
        .map_err(|e| LpError::Internal(format!("PSQ encapsulation failed: {:?}", e)))?;

    // Step 3: Combine ECDH + PSQ via Blake3 KDF
    let mut combined = Vec::with_capacity(64 + psq_psk.len());
    combined.extend_from_slice(ecdh_secret.as_bytes());
    combined.extend_from_slice(&psq_psk); // psq_psk is [u8; 32], need &
    combined.extend_from_slice(salt);

    let final_psk = nym_crypto::kdf::derive_key_blake3(PSK_PSQ_CONTEXT, &combined, &[]);

    // Serialize ciphertext using TLS encoding for transport
    let ct_bytes = ciphertext
        .tls_serialize_detached()
        .map_err(|e| LpError::Internal(format!("Ciphertext serialization failed: {:?}", e)))?;

    Ok((final_psk, ct_bytes))
}

/// Derives a PSK using PSQ (Post-Quantum Secure PSK) protocol - Responder side.
///
/// This function decapsulates the ciphertext from the initiator and combines it with
/// ECDH to derive the same PSK.
///
/// # Formula
/// ```text
/// ecdh_secret = ECDH(local_x25519_private, remote_x25519_public)
/// psq_psk = PSQ_Decapsulate(local_kem_keypair, ciphertext, session_context)
/// psk = Blake3_derive_key(
///     context="nym-lp-psk-psq-v1",
///     input=ecdh_secret || psq_psk || salt
/// )
/// ```
///
/// # Arguments
/// * `local_x25519_private` - Responder's X25519 private key (for Noise)
/// * `remote_x25519_public` - Initiator's X25519 public key (for Noise)
/// * `local_kem_keypair` - Responder's KEM keypair (decapsulation key, public key)
/// * `ciphertext` - PSQ ciphertext from initiator
/// * `salt` - 32-byte salt for session binding
///
/// # Returns
/// * `Ok(psk)` - Derived PSK
/// * `Err(LpError)` - If PSQ decapsulation fails
///
/// # Example
/// ```ignore
/// // Gateway side (after receiving ciphertext)
/// let psk = derive_psk_with_psq_responder(
///     gateway_x25519_private,
///     client_x25519_public,
///     (&gateway_kem_sk, &gateway_kem_pk),
///     &ciphertext,  // from client
///     &salt
/// )?;
/// ```
pub fn derive_psk_with_psq_responder(
    local_x25519_private: &PrivateKey,
    remote_x25519_public: &PublicKey,
    local_kem_keypair: (&DecapsulationKey, &EncapsulationKey),
    ciphertext: &[u8],
    salt: &[u8; 32],
) -> Result<[u8; 32], LpError> {
    // Step 1: Classical ECDH for baseline security
    let ecdh_secret = local_x25519_private.diffie_hellman(remote_x25519_public);

    // Step 2: Extract X25519 keypair from DecapsulationKey/EncapsulationKey
    let (kem_sk, kem_pk) = match (local_kem_keypair.0, local_kem_keypair.1) {
        (DecapsulationKey::X25519(sk), EncapsulationKey::X25519(pk)) => (sk, pk),
        _ => {
            return Err(LpError::KKTError(
                "Only X25519 KEM is currently supported for PSQ".to_string(),
            ))
        }
    };

    // Step 3: Deserialize ciphertext using TLS decoding
    let ct = PsqCiphertext::<PsqX25519>::tls_deserialize(&mut &ciphertext[..])
        .map_err(|e| LpError::Internal(format!("Ciphertext deserialization failed: {:?}", e)))?;

    // Step 4: PSQ decapsulation for post-quantum security
    let psq_psk = PsqX25519::decapsulate_psq(&kem_sk, &kem_pk, &ct, PSQ_SESSION_CONTEXT)
        .map_err(|e| LpError::Internal(format!("PSQ decapsulation failed: {:?}", e)))?;

    // Step 5: Combine ECDH + PSQ via Blake3 KDF (same formula as initiator)
    let mut combined = Vec::with_capacity(64 + psq_psk.len());
    combined.extend_from_slice(ecdh_secret.as_bytes());
    combined.extend_from_slice(&psq_psk); // psq_psk is [u8; 32], need &
    combined.extend_from_slice(salt);

    let final_psk = nym_crypto::kdf::derive_key_blake3(PSK_PSQ_CONTEXT, &combined, &[]);

    Ok(final_psk)
}

/// PSQ protocol wrapper for initiator (client) side.
///
/// Creates a PSQ initiator message with Ed25519 authentication, following the protocol:
/// 1. Encapsulate PSK using responder's KEM key
/// 2. Derive PSK and AEAD keys from K_pq
/// 3. Sign the encapsulation with Ed25519
/// 4. AEAD encrypt (timestamp || signature || public_key)
///
/// Returns (PSK, serialized_payload) where payload includes enc_pq and encrypted auth data.
///
/// # Arguments
/// * `local_x25519_private` - Client's X25519 private key (for hybrid ECDH)
/// * `remote_x25519_public` - Gateway's X25519 public key (for hybrid ECDH)
/// * `remote_kem_public` - Gateway's PQ KEM public key (from KKT)
/// * `client_ed25519_sk` - Client's Ed25519 signing key
/// * `client_ed25519_pk` - Client's Ed25519 public key (credential)
/// * `salt` - Session salt
/// * `session_context` - Context bytes for PSQ (e.g., b"nym-lp-psq-session")
///
/// # Returns
/// `(psk, psq_payload_bytes)` - PSK for Noise and serialized PSQ payload to embed
pub fn psq_initiator_create_message(
    local_x25519_private: &PrivateKey,
    remote_x25519_public: &PublicKey,
    remote_kem_public: &EncapsulationKey,
    client_ed25519_sk: &ed25519::PrivateKey,
    client_ed25519_pk: &ed25519::PublicKey,
    salt: &[u8; 32],
    session_context: &[u8],
) -> Result<([u8; 32], Vec<u8>), LpError> {
    // Step 1: Classical ECDH for baseline security
    let ecdh_secret = local_x25519_private.diffie_hellman(remote_x25519_public);

    // Step 2: PSQ v1 with Ed25519 authentication
    // Extract X25519 KEM key from EncapsulationKey
    let kem_pk = match remote_kem_public {
        EncapsulationKey::X25519(pk) => pk,
        _ => {
            return Err(LpError::KKTError(
                "Only X25519 KEM is currently supported for PSQ".to_string(),
            ))
        }
    };

    // Convert nym Ed25519 keys to libcrux format
    type Ed25519VerificationKey = <Ed25519 as Authenticator>::VerificationKey;
    let ed25519_sk_bytes = client_ed25519_sk.to_bytes();
    let ed25519_pk_bytes = client_ed25519_pk.to_bytes();
    let ed25519_verification_key = Ed25519VerificationKey::from_bytes(ed25519_pk_bytes);

    // Use PSQ v1 API with Ed25519 authentication
    let mut rng = rand09::rng();
    let (state, initiator_msg) = Initiator::send_initial_message::<Ed25519, PsqX25519>(
        session_context,
        Duration::from_secs(3600), // 1 hour expiry
        kem_pk,
        &ed25519_sk_bytes,
        &ed25519_verification_key,
        &mut rng,
    )
    .map_err(|e| LpError::Internal(format!("PSQ v1 send_initial_message failed: {:?}", e)))?;

    // Extract PSQ shared secret (unregistered PSK)
    let psq_psk = state.unregistered_psk();

    // Step 3: Combine ECDH + PSQ via Blake3 KDF
    let mut combined = Vec::with_capacity(64 + psq_psk.len());
    combined.extend_from_slice(ecdh_secret.as_bytes());
    combined.extend_from_slice(psq_psk); // psq_psk is already a &[u8; 32]
    combined.extend_from_slice(salt);

    let final_psk = nym_crypto::kdf::derive_key_blake3(PSK_PSQ_CONTEXT, &combined, &[]);

    // Serialize InitiatorMsg with TLS encoding for transport
    let msg_bytes = initiator_msg
        .tls_serialize_detached()
        .map_err(|e| LpError::Internal(format!("InitiatorMsg serialization failed: {:?}", e)))?;

    Ok((final_psk, msg_bytes))
}

/// PSQ protocol wrapper for responder (gateway) side.
///
/// Processes a PSQ initiator message, verifies authentication, and derives PSK.
/// Follows the protocol:
/// 1. Decapsulate to get K_pq
/// 2. Derive AEAD keys and verify encrypted auth data
/// 3. Verify Ed25519 signature
/// 4. Check timestamp validity
/// 5. Derive PSK
///
/// # Arguments
/// * `local_x25519_private` - Gateway's X25519 private key (for hybrid ECDH)
/// * `remote_x25519_public` - Client's X25519 public key (for hybrid ECDH)
/// * `local_kem_keypair` - Gateway's PQ KEM keypair
/// * `initiator_ed25519_pk` - Client's Ed25519 public key (for signature verification)
/// * `psq_payload` - Serialized PSQ payload from initiator
/// * `salt` - Session salt (must match initiator's)
/// * `session_context` - Context bytes for PSQ
///
/// # Returns
/// `psk` - Derived PSK for Noise
pub fn psq_responder_process_message(
    local_x25519_private: &PrivateKey,
    remote_x25519_public: &PublicKey,
    local_kem_keypair: (&DecapsulationKey, &EncapsulationKey),
    initiator_ed25519_pk: &ed25519::PublicKey,
    psq_payload: &[u8],
    salt: &[u8; 32],
    session_context: &[u8],
) -> Result<[u8; 32], LpError> {
    // Step 1: Classical ECDH for baseline security
    let ecdh_secret = local_x25519_private.diffie_hellman(remote_x25519_public);

    // Step 2: Extract X25519 keypair from DecapsulationKey/EncapsulationKey
    let (kem_sk, kem_pk) = match (local_kem_keypair.0, local_kem_keypair.1) {
        (DecapsulationKey::X25519(sk), EncapsulationKey::X25519(pk)) => (sk, pk),
        _ => {
            return Err(LpError::KKTError(
                "Only X25519 KEM is currently supported for PSQ".to_string(),
            ))
        }
    };

    // Step 3: Deserialize InitiatorMsg using TLS decoding
    let initiator_msg = InitiatorMsg::<PsqX25519>::tls_deserialize(&mut &psq_payload[..])
        .map_err(|e| {
            LpError::Internal(format!("InitiatorMsg deserialization failed: {:?}", e))
        })?;

    // Step 4: Convert nym Ed25519 public key to libcrux VerificationKey format
    type Ed25519VerificationKey = <Ed25519 as Authenticator>::VerificationKey;
    let initiator_ed25519_pk_bytes = initiator_ed25519_pk.to_bytes();
    let initiator_verification_key = Ed25519VerificationKey::from_bytes(initiator_ed25519_pk_bytes);

    // Step 5: PSQ v1 responder processing with Ed25519 verification
    // Note: We use a dummy handle since we don't use the ResponderMsg in this protocol
    let (registered_psk, _responder_msg) = Responder::send::<Ed25519, PsqX25519>(
        b"nym-lp-handle",       // PSK storage handle (unused in our protocol)
        Duration::from_secs(3600), // 1 hour expiry (must match initiator)
        session_context,        // Must match initiator's session_context
        kem_pk,                 // Responder's public key
        kem_sk,                 // Responder's secret key
        &initiator_verification_key, // Initiator's Ed25519 public key for verification
        &initiator_msg,         // InitiatorMsg to verify and process
    )
    .map_err(|e| LpError::Internal(format!("PSQ v1 responder send failed: {:?}", e)))?;

    // Extract the PSQ PSK from the registered PSK
    let psq_psk = registered_psk.psk;

    // Step 6: Combine ECDH + PSQ via Blake3 KDF (same formula as initiator)
    let mut combined = Vec::with_capacity(64 + psq_psk.len());
    combined.extend_from_slice(ecdh_secret.as_bytes());
    combined.extend_from_slice(&psq_psk); // psq_psk is [u8; 32], need &
    combined.extend_from_slice(salt);

    let final_psk = nym_crypto::kdf::derive_key_blake3(PSK_PSQ_CONTEXT, &combined, &[]);

    Ok(final_psk)
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
        let psk1 = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt);
        let psk2 = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt);

        assert_eq!(psk1, psk2, "Same inputs should produce same PSK");
    }

    #[test]
    fn test_psk_derivation_is_symmetric() {
        let keypair_1 = Keypair::default();
        let keypair_2 = Keypair::default();
        let salt = [2u8; 32];

        // Client derives PSK
        let client_psk = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt);

        // Gateway derives PSK from their perspective
        let gateway_psk = derive_psk(keypair_2.private_key(), keypair_1.public_key(), &salt);

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

        #[allow(deprecated)]
        let psk = derive_psk(keypair_1.private_key(), keypair_2.public_key(), &salt);

        assert_eq!(psk.len(), 32, "PSK should be exactly 32 bytes");
    }

    // PSQ-enhanced PSK tests
    use nym_kkt::ciphersuite::{EncapsulationKey, DecapsulationKey, KEM};
    use nym_kkt::key_utils::generate_keypair_libcrux;

    #[test]
    fn test_psq_derivation_deterministic() {
        let mut rng = rand09::rng();

        // Generate X25519 keypairs for Noise
        let client_keypair = Keypair::default();
        let gateway_keypair = Keypair::default();

        // Generate KEM keypair for PSQ
        let (kem_sk, kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let enc_key = EncapsulationKey::X25519(kem_pk);
        let dec_key = DecapsulationKey::X25519(kem_sk);

        let salt = [1u8; 32];

        // Derive PSK twice with same inputs (initiator side)
        let (psk1, ct1) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key,
            &salt,
        )
        .unwrap();

        let (psk2, ct2) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key,
            &salt,
        )
        .unwrap();

        // PSKs will be different due to randomness in PSQ, but ciphertexts too
        // This test verifies the function is deterministic given the SAME ciphertext
        let psk_responder1 = derive_psk_with_psq_responder(
            gateway_keypair.private_key(),
            client_keypair.public_key(),
            (&dec_key, &enc_key),
            &ct1,
            &salt,
        )
        .unwrap();

        let psk_responder2 = derive_psk_with_psq_responder(
            gateway_keypair.private_key(),
            client_keypair.public_key(),
            (&dec_key, &enc_key),
            &ct1,  // Same ciphertext
            &salt,
        )
        .unwrap();

        assert_eq!(
            psk_responder1, psk_responder2,
            "Same ciphertext should produce same PSK"
        );
    }

    #[test]
    fn test_psq_derivation_symmetric() {
        let mut rng = rand09::rng();

        // Generate X25519 keypairs for Noise
        let client_keypair = Keypair::default();
        let gateway_keypair = Keypair::default();

        // Generate KEM keypair for PSQ
        let (kem_sk, kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let enc_key = EncapsulationKey::X25519(kem_pk);
        let dec_key = DecapsulationKey::X25519(kem_sk);

        let salt = [2u8; 32];

        // Client derives PSK (initiator)
        let (client_psk, ciphertext) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key,
            &salt,
        )
        .unwrap();

        // Gateway derives PSK from ciphertext (responder)
        let gateway_psk = derive_psk_with_psq_responder(
            gateway_keypair.private_key(),
            client_keypair.public_key(),
            (&dec_key, &enc_key),
            &ciphertext,
            &salt,
        )
        .unwrap();

        assert_eq!(
            client_psk, gateway_psk,
            "Both sides should derive identical PSK via PSQ"
        );
    }

    #[test]
    fn test_different_kem_keys_different_psk() {
        let mut rng = rand09::rng();

        let client_keypair = Keypair::default();
        let gateway_keypair = Keypair::default();

        // Two different KEM keypairs
        let (_, kem_pk1) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let (_, kem_pk2) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();

        let enc_key1 = EncapsulationKey::X25519(kem_pk1);
        let enc_key2 = EncapsulationKey::X25519(kem_pk2);

        let salt = [3u8; 32];

        let (psk1, _) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key1,
            &salt,
        )
        .unwrap();

        let (psk2, _) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key2,
            &salt,
        )
        .unwrap();

        assert_ne!(
            psk1, psk2,
            "Different KEM keys should produce different PSKs"
        );
    }

    #[test]
    fn test_psq_psk_output_length() {
        let mut rng = rand09::rng();

        let client_keypair = Keypair::default();
        let gateway_keypair = Keypair::default();

        let (_, kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let enc_key = EncapsulationKey::X25519(kem_pk);

        let salt = [4u8; 32];

        let (psk, _) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key,
            &salt,
        )
        .unwrap();

        assert_eq!(psk.len(), 32, "PSQ PSK should be exactly 32 bytes");
    }

    #[test]
    fn test_psq_different_salts_different_psks() {
        let mut rng = rand09::rng();

        let client_keypair = Keypair::default();
        let gateway_keypair = Keypair::default();

        let (_, kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let enc_key = EncapsulationKey::X25519(kem_pk);

        let salt1 = [1u8; 32];
        let salt2 = [2u8; 32];

        let (psk1, _) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key,
            &salt1,
        )
        .unwrap();

        let (psk2, _) = derive_psk_with_psq_initiator(
            client_keypair.private_key(),
            gateway_keypair.public_key(),
            &enc_key,
            &salt2,
        )
        .unwrap();

        assert_ne!(psk1, psk2, "Different salts should produce different PSKs");
    }
}
