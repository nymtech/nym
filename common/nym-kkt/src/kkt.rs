// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Convenience wrappers around KKT protocol functions for easier integration.
//!
//! This module provides simplified APIs for the common use case of exchanging
//! KEM public keys between a client (initiator) and gateway (responder).
//!
//! The underlying KKT protocol is implemented in the `session` module.

use nym_crypto::asymmetric::ed25519;
use rand::{CryptoRng, RngCore};

use crate::{
    ciphersuite::{Ciphersuite, EncapsulationKey},
    context::{KKTContext, KKTMode},
    encryption::{decrypt_initial_kkt_frame, decrypt_kkt_frame, encrypt_kkt_frame},
    error::KKTError,
};

// Re-export core session functions for advanced use cases
pub use crate::session::{
    anonymous_initiator_process, initiator_ingest_response, initiator_process,
    responder_ingest_message, responder_process,
};

use crate::encryption::{KKTSessionSecret, encrypt_initial_kkt_frame};
use crate::frame::KKTFrame;

pub(crate) const KKT_RESPONSE_AAD: &[u8] = b"KKT_Response";
pub(crate) const KKT_INITIAL_FRAME_AAD: &[u8] = b"KKT_INITIAL_FRAME";

/// Perform an *Encrypted* request for a KEM public key from a responder (OneWay mode).
///
/// This is the client-side operation that initiates a KKT exchange.
/// The request will be signed with the provided signing key.
///
/// # Arguments
/// * `rng` - Random number generator
/// * `ciphersuite` - Negotiated ciphersuite (KEM, hash, signature algorithms)
/// * `signing_key` - Client's Ed25519 signing key for authentication
/// * `responder_dh_public_key` - Responder's long-term x25519 Diffie-Hellman public key
///
/// # Returns
/// * `KKTSessionSecret` - Session Secret Key to use when decrypting responses
/// * `KKTContext` - Context to use when validating the response
/// * `Vec<u8>` - Contains the client's ephemeral public key and encrypted and signed bytes to send to responder
///
/// # Example
/// ```ignore
/// let (session_secret, context, request_frame) = request_kem_key(
///     &mut rng,
///     ciphersuite,
///     client_signing_key,
///     responder_dh_public_key,
/// )?;
/// // Send request_frame to gateway
/// ```
pub fn request_kem_key<R: CryptoRng + RngCore>(
    rng: &mut R,
    ciphersuite: Ciphersuite,
    signing_key: &ed25519::PrivateKey,
    responder_dh_public_key: &nym_sphinx::PublicKey,
) -> Result<(KKTSessionSecret, KKTContext, Vec<u8>), KKTError> {
    // OneWay mode: client only wants responder's KEM key
    // None: client doesn't send their own KEM key
    let (initiator_context, initiator_frame) =
        initiator_process(rng, KKTMode::OneWay, ciphersuite, signing_key, None)?;

    // Generate the session's shared secret and encrypt the Initiator's request
    let (session_secret, encrypted_request_bytes) =
        encrypt_initial_kkt_frame(rng, responder_dh_public_key, &initiator_frame)?;

    Ok((session_secret, initiator_context, encrypted_request_bytes))
}

/// Decrypt, validate an *Encrypted* KKT response and extract the responder's KEM public key.
///
/// This is the client-side operation that processes the gateway's response.
/// It verifies the signature and validates the key hash against the expected value
/// (typically retrieved from a directory service).
///
/// # Arguments
/// * `context` - Context from the initial request
/// * `session_secret` - Session Secret Key (generated with request)
/// * `responder_vk` - Responder's Ed25519 verification key (from directory)
/// * `expected_key_hash` - Expected hash of responder's KEM key (from directory)
/// * `response_bytes` - Serialized response frame from responder
///
/// # Returns
/// * `EncapsulationKey` - Authenticated KEM public key of the responder
///
/// # Example
/// ```ignore
/// let gateway_kem_key = validate_kem_response(
///     &mut context,
///     &session_secret,
///     &gateway_verification_key,
///     &expected_hash_from_directory,
///     &response_bytes,
/// )?;
/// // Use gateway_kem_key for PSQ
/// ```
pub fn validate_kem_response<'a>(
    context: &mut KKTContext,
    session_secret: &KKTSessionSecret,
    responder_vk: &ed25519::PublicKey,
    expected_key_hash: &[u8],
    encrypted_response_bytes: &[u8],
) -> Result<EncapsulationKey<'a>, KKTError> {
    let (responder_frame, responder_context) =
        decrypt_kkt_response_frame(session_secret, encrypted_response_bytes)?;

    initiator_ingest_response(
        context,
        &responder_frame,
        &responder_context,
        responder_vk,
        expected_key_hash,
    )
}

/// Decrypts and validates an *Encrypted* KKT response
///
/// This is the client-side operation that processes the gateway's response.
pub fn decrypt_kkt_response_frame(
    session_secret: &KKTSessionSecret,
    frame_ciphertext: &[u8],
) -> Result<(KKTFrame, KKTContext), KKTError> {
    decrypt_kkt_frame(session_secret, frame_ciphertext, KKT_RESPONSE_AAD)
}

/// Handle an *Encrypted* KKT request and generate a signed response with the responder's KEM key.
///
/// This is the gateway-side operation that processes a client's KKT request.
/// It validates the request signature (if authenticated) and responds with
/// the gateway's KEM public key, signed for authenticity.
///
/// # Arguments
/// * `encrypted_request_bytes` - encrypted KEM request
/// * `initiator_vk` - Initiator's Ed25519 verification key (None for anonymous)
/// * `responder_signing_key` - Gateway's Ed25519 signing key
/// * `responder_dh_public_key` - Gateway's long-term x25519 Diffie-Hellman private key
/// * `responder_kem_key` - Gateway's KEM public key to send
///
/// # Returns
/// * `KKTFrame` - Signed response frame containing the KEM public key
///
/// # Example
/// ```ignore
/// let response_frame = handle_kem_request(
///     &request_frame,
///     Some(client_verification_key),  // or None for anonymous
///     gateway_signing_key,
///     &gateway_kem_public_key,
/// )?;
/// // Send response_frame back to client
/// ```
pub fn handle_kem_request<'a, R>(
    rng: &mut R,
    encrypted_request_bytes: &[u8],
    initiator_vk: Option<&ed25519::PublicKey>,
    responder_signing_key: &ed25519::PrivateKey,
    responder_dh_private_key: &nym_sphinx::PrivateKey,
    responder_kem_key: &EncapsulationKey<'a>,
) -> Result<Vec<u8>, KKTError>
where
    R: RngCore + CryptoRng,
{
    // Compute the session's shared secret, decrypt and parse context from the request frame

    let (session_secret, request_frame, initiator_context) =
        decrypt_initial_kkt_frame(responder_dh_private_key, encrypted_request_bytes)?;

    // Validate the request (verifies signature if initiator_vk provided)
    let (mut response_context, _) = responder_ingest_message(
        &initiator_context,
        initiator_vk,
        None, // Not checking initiator's KEM key in OneWay mode
        &request_frame,
    )?;

    // Generate signed response with our KEM public key
    let responder_frame = responder_process(
        &mut response_context,
        request_frame.session_id_ref(),
        responder_signing_key,
        responder_kem_key,
    )?;

    // Encrypt the responder's response with the session's shared secret
    encrypt_kkt_frame(rng, &session_secret, &responder_frame, KKT_RESPONSE_AAD)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{
//         ciphersuite::{HashFunction, KEM, SignatureScheme},
//         key_utils::{generate_keypair_libcrux, hash_encapsulation_key},
//     };

//     #[test]
//     fn test_kkt_wrappers_oneway_authenticated() {
//         let mut rng = rand::rng();

//         // Generate Ed25519 keypairs for both parties
//         let mut initiator_secret = [0u8; 32];
//         rng.fill_bytes(&mut initiator_secret);
//         let initiator_keypair = ed25519::KeyPair::from_secret(initiator_secret, 0);

//         let mut responder_secret = [0u8; 32];
//         rng.fill_bytes(&mut responder_secret);
//         let responder_keypair = ed25519::KeyPair::from_secret(responder_secret, 1);

//         // Generate responder's KEM keypair (X25519 for testing)
//         let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
//         let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

//         // Create ciphersuite
//         let ciphersuite = Ciphersuite::resolve_ciphersuite(
//             KEM::X25519,
//             HashFunction::Blake3,
//             SignatureScheme::Ed25519,
//             None,
//         )
//         .unwrap();

//         // Hash the KEM key (simulating directory storage)
//         let key_hash = hash_encapsulation_key(
//             &ciphersuite.hash_function(),
//             ciphersuite.hash_len(),
//             &responder_kem_key.encode(),
//         );

//         // Client: Request KEM key
//         let (mut context, request_frame) =
//             request_kem_key(&mut rng, ciphersuite, initiator_keypair.private_key()).unwrap();

//         // Gateway: Handle request
//         let response_frame = handle_kem_request(
//             &request_frame,
//             Some(initiator_keypair.public_key()), // Authenticated
//             responder_keypair.private_key(),
//             &responder_kem_key,
//         )
//         .unwrap();

//         // Client: Validate response
//         let obtained_key = validate_kem_response(
//             &mut context,
//             responder_keypair.public_key(),
//             &key_hash,
//             &response_frame.to_bytes(),
//         )
//         .unwrap();

//         // Verify we got the correct KEM key
//         assert_eq!(obtained_key.encode(), responder_kem_key.encode());
//     }

//     #[test]
//     fn test_kkt_wrappers_anonymous() {
//         let mut rng = rand::rng();

//         // Only responder has keys
//         let mut responder_secret = [0u8; 32];
//         rng.fill_bytes(&mut responder_secret);
//         let responder_keypair = ed25519::KeyPair::from_secret(responder_secret, 1);

//         let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
//         let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

//         let ciphersuite = Ciphersuite::resolve_ciphersuite(
//             KEM::X25519,
//             HashFunction::Blake3,
//             SignatureScheme::Ed25519,
//             None,
//         )
//         .unwrap();

//         let key_hash = hash_encapsulation_key(
//             &ciphersuite.hash_function(),
//             ciphersuite.hash_len(),
//             &responder_kem_key.encode(),
//         );

//         // Anonymous initiator
//         let (mut context, request_frame) =
//             anonymous_initiator_process(&mut rng, ciphersuite).unwrap();

//         // Gateway: Handle anonymous request
//         let response_frame = handle_kem_request(
//             &request_frame,
//             None, // Anonymous - no verification key
//             responder_keypair.private_key(),
//             &responder_kem_key,
//         )
//         .unwrap();

//         // Initiator: Validate response
//         let obtained_key = validate_kem_response(
//             &mut context,
//             responder_keypair.public_key(),
//             &key_hash,
//             &response_frame.to_bytes(),
//         )
//         .unwrap();

//         assert_eq!(obtained_key.encode(), responder_kem_key.encode());
//     }

//     #[test]
//     fn test_invalid_signature_rejected() {
//         let mut rng = rand::rng();

//         let mut initiator_secret = [0u8; 32];
//         rng.fill_bytes(&mut initiator_secret);
//         let initiator_keypair = ed25519::KeyPair::from_secret(initiator_secret, 0);

//         let mut responder_secret = [0u8; 32];
//         rng.fill_bytes(&mut responder_secret);
//         let responder_keypair = ed25519::KeyPair::from_secret(responder_secret, 1);

//         // Different keypair for wrong signature
//         let mut wrong_secret = [0u8; 32];
//         rng.fill_bytes(&mut wrong_secret);
//         let wrong_keypair = ed25519::KeyPair::from_secret(wrong_secret, 2);

//         let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
//         let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

//         let ciphersuite = Ciphersuite::resolve_ciphersuite(
//             KEM::X25519,
//             HashFunction::Blake3,
//             SignatureScheme::Ed25519,
//             None,
//         )
//         .unwrap();

//         let (_context, request_frame) =
//             request_kem_key(&mut rng, ciphersuite, initiator_keypair.private_key()).unwrap();

//         // Gateway handles request but we provide WRONG verification key
//         let result = handle_kem_request(
//             &request_frame,
//             Some(wrong_keypair.public_key()), // Wrong key!
//             responder_keypair.private_key(),
//             &responder_kem_key,
//         );

//         // Should fail signature verification
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_hash_mismatch_rejected() {
//         let mut rng = rand::rng();

//         let mut initiator_secret = [0u8; 32];
//         rng.fill_bytes(&mut initiator_secret);
//         let initiator_keypair = ed25519::KeyPair::from_secret(initiator_secret, 0);

//         let mut responder_secret = [0u8; 32];
//         rng.fill_bytes(&mut responder_secret);
//         let responder_keypair = ed25519::KeyPair::from_secret(responder_secret, 1);

//         let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
//         let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

//         let ciphersuite = Ciphersuite::resolve_ciphersuite(
//             KEM::X25519,
//             HashFunction::Blake3,
//             SignatureScheme::Ed25519,
//             None,
//         )
//         .unwrap();

//         // Use WRONG hash
//         let wrong_hash = [0u8; 32];

//         let (mut context, request_frame) =
//             request_kem_key(&mut rng, ciphersuite, initiator_keypair.private_key()).unwrap();

//         let response_frame = handle_kem_request(
//             &request_frame,
//             Some(initiator_keypair.public_key()),
//             responder_keypair.private_key(),
//             &responder_kem_key,
//         )
//         .unwrap();

//         // Client validates with WRONG hash
//         let result = validate_kem_response(
//             &mut context,
//             responder_keypair.public_key(),
//             &wrong_hash, // Wrong!
//             &response_frame.to_bytes(),
//         );

//         // Should fail hash validation
//         assert!(result.is_err());
//     }
// }
