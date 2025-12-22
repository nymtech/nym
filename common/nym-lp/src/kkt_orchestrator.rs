// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! KKT (Key Encapsulation Transport) orchestration for nym-lp sessions.
//!
//! This module provides functions to perform KKT key exchange before establishing
//! an nym-lp session. The KKT protocol allows secure distribution of post-quantum
//! KEM public keys, which are then used with PSQ to derive a strong pre-shared key
//! for the Noise protocol.
//!
//! # Protocol Flow
//!
//! 1. **Client (Initiator)**:
//!    - Calls `create_request()` to generate a KKT request
//!    - Sends `LpMessage::KKTRequest` to gateway
//!    - Receives `LpMessage::KKTResponse` from gateway
//!    - Calls `process_response()` to validate and extract gateway's KEM key
//!
//! 2. **Gateway (Responder)**:
//!    - Receives `LpMessage::KKTRequest` from client
//!    - Calls `handle_request()` to validate request and generate response
//!    - Sends `LpMessage::KKTResponse` to client
//!
//! # Example
//!
//! ```ignore
//! use nym_lp::kkt_orchestrator::{create_request, process_response, handle_request};
//! use nym_lp::message::{KKTRequestData, KKTResponseData};
//! use nym-kkt::ciphersuite::{Ciphersuite, KEM, HashFunction, SignatureScheme, EncapsulationKey};
//!
//! // Setup ciphersuite
//! let ciphersuite = Ciphersuite::resolve_ciphersuite(
//!     KEM::X25519,
//!     HashFunction::Blake3,
//!     SignatureScheme::Ed25519,
//!     None,
//! ).unwrap();
//!
//! // Client: Create request
//! let (session_secret, client_context, request_data) = create_request(
//!     ciphersuite,
//!     &client_signing_key,
//!     &responder_dh_public_key
//! ).unwrap();
//!
//! // Gateway: Handle request
//! let response_data = handle_request(
//!     &request_data,
//!     Some(&client_verification_key),
//!     &gateway_signing_key,
//!     &gateway_dh_private_key,
//!     &gateway_kem_public_key,
//! ).unwrap();
//!
//! // Client: Process response
//! let gateway_kem_key = process_response(
//!     client_context,
//!     &session_secret,
//!     &gateway_verification_key,
//!     &expected_key_hash,
//!     &response_data,
//! ).unwrap();
//! ```

use crate::LpError;
use crate::message::{KKTRequestData, KKTResponseData};
use nym_crypto::asymmetric::ed25519;
use nym_kkt::ciphersuite::{Ciphersuite, EncapsulationKey};
use nym_kkt::context::KKTContext;
use nym_kkt::encryption::KKTSessionSecret;
use nym_kkt::kkt::{handle_kem_request, request_kem_key, validate_kem_response};

/// Creates a KKT request to obtain the responder's KEM public key.
///
/// This is called by the **client (initiator)** to begin the KKT exchange.
/// The returned context must be used when processing the response.
///
/// # Arguments
/// * `ciphersuite` - Negotiated ciphersuite (KEM, hash, signature algorithms)
/// * `signing_key` - Client's Ed25519 signing key for authentication
/// * `responder_dh_public_key` - Gateway's x25519 public key (from directory)
///
/// # Returns
/// * `KKTSessionSecret` - Session secret key to encrypt/decrypt KKT messages for this session
/// * `KKTContext` - Context to use when validating the response
/// * `KKTRequestData` - Serialized KKT request frame to send to gateway
///
/// # Errors
/// Returns `LpError::KKTError` if KKT request generation fails.
pub fn create_request(
    ciphersuite: Ciphersuite,
    signing_key: &ed25519::PrivateKey,
    responder_dh_public_key: &nym_sphinx::PublicKey,
) -> Result<(KKTSessionSecret, KKTContext, KKTRequestData), LpError> {
    // Note: Uses rand 0.9's thread_rng() to match nym-kkt's rand version
    let mut rng = rand09::rng();
    let (session_secret, context, request_bytes) =
        request_kem_key(&mut rng, ciphersuite, signing_key, &responder_dh_public_key)
            .map_err(|e| LpError::KKTError(e.to_string()))?;

    Ok((session_secret, context, KKTRequestData(request_bytes)))
}

/// Processes a KKT response and extracts the responder's KEM public key.
///
/// This is called by the **client (initiator)** after receiving a KKT response
/// from the gateway. It verifies the signature and validates the key hash.
///
/// # Arguments
/// * `context` - Context from the initial `create_request()` call
/// * `session_secret` - The KKT session secret key from the initial `create_request()` call
/// * `responder_vk` - Responder's Ed25519 verification key (from directory)
/// * `expected_key_hash` - Expected hash of responder's KEM key (from directory)
/// * `response_data` - Serialized KKT response frame from responder
///
/// # Returns
/// * `EncapsulationKey` - Authenticated KEM public key of the responder
///
/// # Errors
/// Returns `LpError::KKTError` if:
/// - Response deserialization fails
/// - Signature verification fails
/// - Key hash doesn't match expected value
pub fn process_response<'a>(
    mut context: KKTContext,
    session_secret: &KKTSessionSecret,
    responder_vk: &ed25519::PublicKey,
    expected_key_hash: &[u8],
    response_data: &KKTResponseData,
) -> Result<EncapsulationKey<'a>, LpError> {
    validate_kem_response(
        &mut context,
        session_secret,
        responder_vk,
        expected_key_hash,
        &response_data.0,
    )
    .map_err(|e| LpError::KKTError(e.to_string()))
}

/// Handles a KKT request and generates a signed response with the responder's KEM key.
///
/// This is called by the **gateway (responder)** when receiving a KKT request
/// from a client. It validates the request signature (if authenticated) and
/// responds with the gateway's KEM public key, signed for authenticity.
///
/// # Arguments
/// * `request_data` - Serialized KKT request frame from initiator
/// * `initiator_vk` - Initiator's Ed25519 verification key (None for anonymous)
/// * `responder_signing_key` - Gateway's Ed25519 signing key
/// * `responder_dh_private_key` - Gateway's x25519 private key
/// * `responder_kem_key` - Gateway's KEM public key to send
///
/// # Returns
/// * `KKTResponseData` - Signed response frame containing the KEM public key
///
/// # Errors
/// Returns `LpError::KKTError` if:
/// - Request deserialization fails
/// - Signature verification fails (if authenticated)
/// - Response generation fails
pub fn handle_request<'a>(
    request_data: &KKTRequestData,
    initiator_vk: Option<&ed25519::PublicKey>,
    responder_signing_key: &ed25519::PrivateKey,
    responder_dh_private_key: &nym_sphinx::PrivateKey,
    responder_kem_key: &EncapsulationKey<'a>,
) -> Result<KKTResponseData, LpError> {
    let mut rng = rand09::rng();
    // Handle the request and generate response
    let response_bytes = handle_kem_request(
        &mut rng,
        &request_data.0,
        initiator_vk,
        responder_signing_key,
        responder_dh_private_key,
        responder_kem_key,
    )
    .map_err(|e| LpError::KKTError(e.to_string()))?;

    Ok(KKTResponseData(response_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_kkt::ciphersuite::{HashFunction, KEM, SignatureScheme};
    use nym_kkt::key_utils::{
        generate_keypair_ed25519, generate_keypair_libcrux, generate_keypair_x25519,
        hash_encapsulation_key,
    };
    use nym_kkt::kkt::initiator_ingest_response;
    use rand09::RngCore;

    #[test]
    fn test_kkt_roundtrip_authenticated() {
        let mut rng = rand09::rng();

        // Generate Ed25519 keypairs for both parties
        let initiator_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(0));
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

        let (responder_x25519_sk, responder_x25519_pk) = generate_keypair_x25519();

        // Generate responder's KEM keypair (X25519 for testing)
        let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

        // Create ciphersuite
        let ciphersuite = Ciphersuite::resolve_ciphersuite(
            KEM::X25519,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            None,
        )
        .unwrap();

        // Hash the KEM key (simulating directory storage)
        let key_hash = hash_encapsulation_key(
            &ciphersuite.hash_function(),
            ciphersuite.hash_len(),
            &responder_kem_key.encode(),
        );

        // Client: Create request
        let (session_secret, context, request_data) = create_request(
            ciphersuite,
            initiator_ed25519_keypair.private_key(),
            &responder_x25519_pk,
        )
        .unwrap();

        // Gateway: Handle request
        let response_data = handle_request(
            &request_data,
            Some(initiator_ed25519_keypair.public_key()),
            responder_ed25519_keypair.private_key(),
            &responder_x25519_sk,
            &responder_kem_key,
        )
        .unwrap();

        // Client: Process response
        let obtained_key = process_response(
            context,
            &session_secret,
            responder_ed25519_keypair.public_key(),
            &key_hash,
            &response_data,
        )
        .unwrap();

        // Verify we got the correct KEM key
        assert_eq!(obtained_key.encode(), responder_kem_key.encode());
    }

    // #[test]
    // fn test_kkt_roundtrip_anonymous() {
    //     let mut rng = rand09::rng();

    //     // Only responder has keys (anonymous initiator)
    //     // Generate Ed25519 keypairs for both parties

    //     let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

    //     let (responder_x25519_sk, responder_x25519_pk) = generate_keypair_x25519();

    //     let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
    //     let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

    //     let ciphersuite = Ciphersuite::resolve_ciphersuite(
    //         KEM::X25519,
    //         HashFunction::Blake3,
    //         SignatureScheme::Ed25519,
    //         None,
    //     )
    //     .unwrap();

    //     let key_hash = hash_encapsulation_key(
    //         &ciphersuite.hash_function(),
    //         ciphersuite.hash_len(),
    //         &responder_kem_key.encode(),
    //     );

    //     // Anonymous initiator - use anonymous_initiator_process directly
    //     use nym_kkt::kkt::anonymous_initiator_process;
    //     let (mut context, request_frame) =
    //         anonymous_initiator_process(&mut rng, ciphersuite).unwrap();
    //     let request_data = KKTRequestData(request_frame.to_bytes());

    //     // Gateway: Handle anonymous request
    //     let response_data = handle_request(
    //         &request_data,
    //         None,
    //         responder_ed25519_keypair.private_key(),
    //         &responder_x25519_sk,
    //         &responder_kem_key,
    //     )
    //     .unwrap();

    //     // Initiator: Validate response
    //     let obtained_key = initiator_ingest_response(
    //         &mut context,
    //         responder_ed25519_keypair.public_key(),
    //         &key_hash,
    //         &response_data.0,
    //     )
    //     .unwrap();

    //     assert_eq!(obtained_key.encode(), responder_kem_key.encode());
    // }

    #[test]
    fn test_invalid_signature_rejected() {
        let mut rng = rand09::rng();

        // Generate Ed25519 keypairs for both parties
        let initiator_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(0));
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

        let (responder_x25519_sk, responder_x25519_pk) = generate_keypair_x25519();

        // Different keypair for wrong signature
        let mut wrong_secret = [0u8; 32];
        rng.fill_bytes(&mut wrong_secret);
        let wrong_keypair = ed25519::KeyPair::from_secret(wrong_secret, 2);

        let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

        let ciphersuite = Ciphersuite::resolve_ciphersuite(
            KEM::X25519,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            None,
        )
        .unwrap();

        let (_session_secret, _context, request_data) = create_request(
            ciphersuite,
            initiator_ed25519_keypair.private_key(),
            &responder_x25519_pk,
        )
        .unwrap();

        // Gateway handles request but we provide WRONG verification key
        let result = handle_request(
            &request_data,
            Some(wrong_keypair.public_key()), // Wrong key!
            responder_ed25519_keypair.private_key(),
            &responder_x25519_sk,
            &responder_kem_key,
        );

        // Should fail signature verification
        assert!(result.is_err());
        if let Err(LpError::KKTError(_)) = result {
            // Expected
        } else {
            panic!("Expected KKTError");
        }
    }

    #[test]
    fn test_hash_mismatch_rejected() {
        let mut rng = rand09::rng();

        // Generate Ed25519 keypairs for both parties
        let initiator_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(0));
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

        let (responder_x25519_sk, responder_x25519_pk) = generate_keypair_x25519();

        let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

        let ciphersuite = Ciphersuite::resolve_ciphersuite(
            KEM::X25519,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            None,
        )
        .unwrap();

        // Use WRONG hash
        let wrong_hash = [0u8; 32];

        let (session_secret, context, request_data) = create_request(
            ciphersuite,
            initiator_ed25519_keypair.private_key(),
            &responder_x25519_pk,
        )
        .unwrap();

        let response_data = handle_request(
            &request_data,
            Some(initiator_ed25519_keypair.public_key()),
            responder_ed25519_keypair.private_key(),
            &responder_x25519_sk,
            &responder_kem_key,
        )
        .unwrap();

        // Client validates with WRONG hash
        let result = process_response(
            context,
            &session_secret,
            responder_ed25519_keypair.public_key(),
            &wrong_hash, // Wrong!
            &response_data,
        );

        // Should fail hash validation
        assert!(result.is_err());
        if let Err(LpError::KKTError(_)) = result {
            // Expected
        } else {
            panic!("Expected KKTError");
        }
    }

    #[test]
    fn test_malformed_request_rejected() {
        let mut rng = rand09::rng();

        let mut responder_secret = [0u8; 32];
        rng.fill_bytes(&mut responder_secret);
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

        let (responder_x25519_sk, _responder_x25519_pk) = generate_keypair_x25519();

        let (_, responder_kem_pk) = generate_keypair_libcrux(&mut rng, KEM::X25519).unwrap();
        let responder_kem_key = EncapsulationKey::X25519(responder_kem_pk);

        // Create malformed request data (invalid bytes)
        let malformed_request = KKTRequestData(vec![0xFF; 100]);

        let result = handle_request(
            &malformed_request,
            None,
            responder_ed25519_keypair.private_key(),
            &responder_x25519_sk,
            &responder_kem_key,
        );

        // Should fail to parse
        assert!(result.is_err());
        if let Err(LpError::KKTError(_)) = result {
            // Expected
        } else {
            panic!("Expected KKTError");
        }
    }

    #[test]
    fn test_malformed_response_rejected() {
        let mut rng = rand09::rng();

        // Generate Ed25519 keypairs for both parties
        let initiator_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(0));
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

        let (_responder_x25519_sk, responder_x25519_pk) = generate_keypair_x25519();

        let ciphersuite = Ciphersuite::resolve_ciphersuite(
            KEM::X25519,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            None,
        )
        .unwrap();

        let (session_secret, context, _request_data) = create_request(
            ciphersuite,
            initiator_ed25519_keypair.private_key(),
            &responder_x25519_pk,
        )
        .unwrap();

        // Create malformed response data
        let malformed_response = KKTResponseData(vec![0xFF; 100]);
        let key_hash = [0u8; 32];

        let result = process_response(
            context,
            &session_secret,
            responder_ed25519_keypair.public_key(),
            &key_hash,
            &malformed_response,
        );

        // Should fail to parse
        assert!(result.is_err());
        if let Err(LpError::KKTError(_)) = result {
            // Expected
        } else {
            panic!("Expected KKTError");
        }
    }
}
