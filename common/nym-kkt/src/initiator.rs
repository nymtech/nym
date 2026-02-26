// Copyright 2025-2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use libcrux_psq::handshake::types::DHPublicKey;
use nym_kkt_ciphersuite::Ciphersuite;
use rand09::{CryptoRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::keys::EncapsulationKey;
use crate::message::{KKTRequest, KKTResponse, ProcessedKKTResponse};
use crate::{
    carrier::Carrier,
    context::{KKTContext, KKTMode, KKTRole, KKTStatus},
    error::KKTError,
    frame::KKTFrame,
    key_utils::validate_encapsulation_key,
};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct KKTInitiator<'a> {
    carrier: Carrier,

    #[zeroize(skip)]
    context: KKTContext,

    #[zeroize(skip)]
    expected_hash: &'a [u8],
}

pub struct KKTRequestWithReceiverIndex {
    pub request: KKTRequest,
    pub receiver_index: u64,
}

impl<'a> KKTInitiator<'a> {
    // to be used by clients
    pub fn generate_one_way_request<R>(
        rng: &mut R,
        ciphersuite: Ciphersuite,
        responder_dh_public_key: &DHPublicKey,
        expected_hash: &'a [u8],
        outer_protocol_version: u8,
        payload: Option<Vec<u8>>
    ) -> Result<(Self, KKTRequestWithReceiverIndex), KKTError>
    where
        R: CryptoRng + RngCore,
    {
        Self::generate_encrypted_request(
            rng,
            KKTMode::OneWay,
            ciphersuite,
            None,
            responder_dh_public_key,
            expected_hash,
            outer_protocol_version,
            payload
        )
    }

    // to be used by nodes
    pub fn generate_mutual_request<'b, R>(
        rng: &mut R,
        ciphersuite: Ciphersuite,
        local_encapsulation_key: &'b [u8],
        responder_dh_public_key: &DHPublicKey,
        expected_hash: &'a [u8],
        outer_protocol_version: u8,
        payload: Option<Vec<u8>>,
    ) -> Result<(Self, KKTRequestWithReceiverIndex), KKTError>
    where
        R: CryptoRng + RngCore,
    {
        Self::generate_encrypted_request(
            rng,
            KKTMode::Mutual,
            ciphersuite,
            Some(local_encapsulation_key),
            responder_dh_public_key,
            expected_hash,
            outer_protocol_version,
            payload
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_encrypted_request<'b, R>(
        rng: &mut R,
        mode: KKTMode,
        ciphersuite: Ciphersuite,
        local_encapsulation_key: Option<&'b [u8]>,
        responder_dh_public_key: &DHPublicKey,
        expected_hash: &'a [u8],
        outer_protocol_version: u8,
        payload: Option<Vec<u8>>,
    ) -> Result<(Self, KKTRequestWithReceiverIndex), KKTError>
    where
        R: CryptoRng + RngCore,
    {
        let frame = initiator_process(mode, ciphersuite, local_encapsulation_key, payload)?;
        let context = *frame.context();

        let request =
            frame.encrypt_initiator_frame(rng, responder_dh_public_key, outer_protocol_version)?;

        Ok((
            Self {
                carrier: request.carrier,
                context,
                expected_hash,
            },
            KKTRequestWithReceiverIndex {
                request: request.request,
                receiver_index: request.receiver_index,
            },
        ))
    }

    pub fn process_response(
        &mut self,
        response: KKTResponse,
        response_payload_len: usize
    ) -> Result<ProcessedKKTResponse, KKTError> {
        let decrypted_response_bytes = self.carrier.decrypt(&response.encrypted_frame)?;
        let response_frame = KKTFrame::from_bytes(&decrypted_response_bytes, response_payload_len)?;
        initiator_ingest_response(&self.context, &response_frame, self.expected_hash)
    }
}

pub fn initiator_process(
    mode: KKTMode,
    ciphersuite: Ciphersuite,
    own_encapsulation_key: Option<&[u8]>,
    payload: Option<Vec<u8>>,
) -> Result<KKTFrame, KKTError> {
    let context = KKTContext::new(KKTRole::Initiator, mode, ciphersuite);

    let body: &[u8] = match mode {
        KKTMode::OneWay => &[],
        KKTMode::Mutual => match own_encapsulation_key {
            Some(encaps_key) => encaps_key,

            // Missing key
            None => {
                return Err(KKTError::FunctionInputError {
                    info: "KEM Key Not Provided",
                });
            }
        },
    };

    Ok(KKTFrame::new(
        context,
        body,
        match payload {
            Some(payload_vec) => payload_vec,
            None => Vec::with_capacity(0),
        },
    ))
}

pub fn initiator_ingest_response(
    own_context: &KKTContext,
    remote_frame: &KKTFrame,
    expected_hash: &[u8],
) -> Result<ProcessedKKTResponse, KKTError> {
    let remote_context = remote_frame.context();
    let verified_initiator_kem_key = match remote_context.status() {
        KKTStatus::Ok | KKTStatus::UnverifiedKEMKey => {
            match validate_encapsulation_key(
                own_context.ciphersuite().hash_function(),
                own_context.ciphersuite().hash_len(),
                remote_frame.body_ref(),
                expected_hash,
            ) {
                true => remote_context.status() != KKTStatus::UnverifiedKEMKey,

                // The key does not match the hash obtained from the directory
                false => return Err(KKTError::MismatchedKEMHash),
            }
        }
        _ => {
            return Err(KKTError::ResponderFlaggedError {
                status: remote_context.status(),
            });
        }
    };

    let kem = own_context.ciphersuite().kem();
    let kem_bytes = remote_frame.body_ref();
    let encapsulation_key = EncapsulationKey::try_from_bytes(kem_bytes.to_vec(), kem)?;
    Ok(ProcessedKKTResponse {
        encapsulation_key,
        verified_initiator_kem_key,
        response_payload: remote_frame.payload().to_vec()
    })
}
