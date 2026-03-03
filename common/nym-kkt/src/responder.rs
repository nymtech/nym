// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::key_utils::validate_encapsulation_key;
use crate::keys::{EncapsulationKey, KEMKeys};
use crate::message::{KKTRequest, KKTResponse, ProcessedKKTRequest};
use crate::{
    context::{KKTContext, KKTMode, KKTRole, KKTStatus},
    error::KKTError,
    frame::KKTFrame,
};
use libcrux_psq::handshake::types::DHKeyPair;
use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, KEM, KEMKeyDigests, SignatureScheme};
use std::collections::BTreeMap;

/// Representation of a KKT Responder
pub struct KKTResponder<'a> {
    /// Long-term x25519 DH key pair of this Responder
    x25519_keypair: &'a DHKeyPair,

    /// KEM keys of this responder
    kem_keys: &'a KEMKeys,

    /// Digests of the initiator's kem key
    expected_initiator_kem_digests: &'a BTreeMap<KEM, KEMKeyDigests>,

    /// List of supported Hash Functions by this Responder
    supported_hash_functions: Vec<HashFunction>,

    /// List of supported Signature Schemes by this Responder
    supported_signature_schemes: Vec<SignatureScheme>,

    /// List of supported outer (LP) protocol version by this Responder
    supported_outer_protocol_versions: Vec<u8>,
}

impl<'a> KKTResponder<'a> {
    pub fn new(
        x25519_keypair: &'a DHKeyPair,
        kem_keys: &'a KEMKeys,
        expected_initiator_kem_digests: &'a BTreeMap<KEM, KEMKeyDigests>,
        supported_hash_functions: &[HashFunction],
        supported_signature_schemes: &[SignatureScheme],
        supported_outer_protocol_versions: &[u8],
    ) -> Result<Self, KKTError> {
        if supported_hash_functions.is_empty() {
            return Err(KKTError::FunctionInputError {
                info: "Did not provide a supported HashFunction when instantiating a KKTResponder",
            });
        }

        if supported_signature_schemes.is_empty() {
            return Err(KKTError::FunctionInputError {
                info: "Did not provide a supported SignatureScheme when instantiating a KKTResponder",
            });
        }

        if supported_outer_protocol_versions.is_empty() {
            return Err(KKTError::FunctionInputError {
                info: "Did not provide a supported outer protocol version when instantiating a KKTResponder",
            });
        }

        Ok(Self {
            x25519_keypair,
            kem_keys,
            expected_initiator_kem_digests,
            supported_hash_functions: supported_hash_functions.to_vec(),
            supported_signature_schemes: supported_signature_schemes.to_vec(),
            supported_outer_protocol_versions: supported_outer_protocol_versions.to_vec(),
        })
    }

    /// Attempt to retrieve expected KEM key hash of the initiator based on the received `Ciphersuite`
    pub(crate) fn expected_initiator_kem_digest(
        &self,
        ciphersuite: Ciphersuite,
    ) -> Result<&Vec<u8>, KKTError> {
        let kem = ciphersuite.kem();
        let hash_function = ciphersuite.hash_function();

        self.expected_initiator_kem_digests
            .get(&kem)
            .ok_or(KKTError::NoKnownKEMKeyDigests { kem, hash_function })?
            .get(&hash_function)
            .ok_or(KKTError::NoKnownKEMKeyDigests { kem, hash_function })
    }

    fn check_ciphersuite_compatiblity(
        &self,
        remote_ciphersuite: Ciphersuite,
    ) -> Result<(), KKTError> {
        let r_hash = remote_ciphersuite.hash_function();
        let r_sig = remote_ciphersuite.signature_scheme();

        if !self.supported_hash_functions.contains(&r_hash) {
            return Err(KKTError::IncompatibilityError {
                info: "Unsupported HashFunction",
            });
        }

        if !self.supported_signature_schemes.contains(&r_sig) {
            return Err(KKTError::IncompatibilityError {
                info: "Unsupported SignatureScheme",
            });
        }

        Ok(())
    }

    // When this function fails, we do that silently (i.e. we don't generate a response to the initiator).

    pub fn process_request(
        &self,
        request: KKTRequest,
        request_payload_len: usize,
    ) -> Result<ProcessedKKTRequest, KKTError> {
        let processed_req = KKTFrame::decrypt_initiator_frame(
            self.x25519_keypair,
            request,
            &self.supported_outer_protocol_versions,
            request_payload_len,
        )?;

        let remote_context = *processed_req.remote_context();

        let remote_frame = processed_req.remote_frame;
        let request_payload = remote_frame.payload().to_vec();
        let mut carrier = processed_req.carrier;

        self.check_ciphersuite_compatiblity(remote_context.ciphersuite())?;

        let (local_context, remote_encapsulation_key) = match remote_context.mode() {
            KKTMode::OneWay => responder_ingest_message(None, remote_frame)?,
            KKTMode::Mutual => {
                let digest = self.expected_initiator_kem_digest(remote_context.ciphersuite())?;
                responder_ingest_message(Some(digest), remote_frame)?
            }
        };

        let kem = local_context.ciphersuite().kem();
        let Some(kem_key) = self.kem_keys.encoded_encapsulation_key(kem) else {
            return Err(KKTError::IncompatibilityError {
                info: "Unsupported KEM",
            });
        };

        // for now the response payload is empty
        let response_payload = Vec::new();

        let frame = KKTFrame::new(local_context, kem_key, response_payload);

        // encryption - responder frame
        let encrypted_frame = carrier.encrypt(&frame.try_to_bytes()?)?;
        Ok(ProcessedKKTRequest {
            response: KKTResponse { encrypted_frame },
            remote_encapsulation_key,
            requested_kem: remote_context.ciphersuite().kem(),
            outer_protocol_version: processed_req.outer_protocol_version,
            request_payload,
        })
    }
}

pub fn responder_ingest_message(
    expected_hash: Option<&[u8]>,
    remote_frame: KKTFrame,
) -> Result<(KKTContext, Option<EncapsulationKey>), KKTError> {
    let remote_context = remote_frame.context();
    let mut own_context = remote_context.derive_responder_header()?;
    let cs = own_context.ciphersuite();

    match remote_context.role() {
        KKTRole::Initiator => {
            // using own_context here because maybe for whatever reason we want to ignore the remote kem key
            match own_context.mode() {
                KKTMode::OneWay => Ok((own_context, None)),
                KKTMode::Mutual => {
                    let Some(expected_hash) = expected_hash else {
                        own_context.update_status(KKTStatus::UnverifiedKEMKey);
                        // we don't store an unverified key
                        // changing the status notifies the initiator that we didn't
                        return Ok((own_context, None));
                    };

                    if !validate_encapsulation_key(
                        cs.hash_function(),
                        cs.hash_len(),
                        remote_frame.body_ref(),
                        expected_hash,
                    ) {
                        // The key does not match the hash obtained from the directory
                        return Err(KKTError::MismatchedKEMHash);
                    }
                    let remote_key =
                        EncapsulationKey::try_from_bytes(remote_frame.body(), cs.kem())?;
                    Ok((own_context, Some(remote_key)))
                }
            }
        }

        KKTRole::Responder => Err(KKTError::IncompatibilityError {
            info: "Responder received a request from another responder.",
        }),
    }
}
