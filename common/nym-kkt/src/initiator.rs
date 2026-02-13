use libcrux_psq::handshake::types::DHPublicKey;
use nym_kkt_ciphersuite::Ciphersuite;
use rand09::{CryptoRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{
    carrier::Carrier,
    ciphersuite::EncapsulationKey,
    context::{KKTContext, KKTMode, KKTRole, KKTStatus},
    error::KKTError,
    frame::KKTFrame,
    key_utils::validate_encapsulation_key,
};

pub struct KKTInitiator<'a> {
    carrier: Carrier,
    context: KKTContext,
    expected_hash: &'a [u8],
}
impl<'a> Zeroize for KKTInitiator<'a> {
    fn zeroize(&mut self) {
        self.carrier.zeroize();
    }
}
impl<'a> ZeroizeOnDrop for KKTInitiator<'a> {}

impl<'a> KKTInitiator<'a> {
    // to be used by clients
    pub fn generate_one_way_request<R>(
        rng: &mut R,
        ciphersuite: &Ciphersuite,
        responder_dh_public_key: &DHPublicKey,
        expected_hash: &'a [u8],
        outer_protocol_version: u8,
    ) -> Result<(Self, Vec<u8>), KKTError>
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
        )
    }

    // to be used by nodes
    pub fn generate_mutual_request<R>(
        rng: &mut R,
        ciphersuite: &Ciphersuite,
        local_encapsulation_key: &EncapsulationKey,
        responder_dh_public_key: &DHPublicKey,
        expected_hash: &'a [u8],
        outer_protocol_version: u8,
    ) -> Result<(Self, Vec<u8>), KKTError>
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
        )
    }

    fn generate_encrypted_request<R>(
        rng: &mut R,
        mode: KKTMode,
        ciphersuite: &Ciphersuite,
        local_encapsulation_key: Option<&EncapsulationKey>,
        responder_dh_public_key: &DHPublicKey,
        expected_hash: &'a [u8],
        outer_protocol_version: u8,
    ) -> Result<(Self, Vec<u8>), KKTError>
    where
        R: CryptoRng + RngCore,
    {
        let (context, frame) = initiator_process(mode, ciphersuite, local_encapsulation_key)?;
        let (carrier, message_bytes) =
            frame.encrypt_initiator_frame(rng, responder_dh_public_key, outer_protocol_version)?;

        Ok((
            Self {
                carrier,
                context,
                expected_hash,
            },
            message_bytes,
        ))
    }

    // bool would be true if the initiator was using mutual mode
    // and the responder was able to verify the initiator's kem key
    pub fn process_response(
        &mut self,
        response_bytes: &[u8],
    ) -> Result<(EncapsulationKey, bool), KKTError> {
        let decrypted_response_bytes = self.carrier.decrypt(response_bytes)?;
        let (response_frame, remote_context) = KKTFrame::from_bytes(&decrypted_response_bytes)?;
        initiator_ingest_response(
            &mut self.context,
            &response_frame,
            &remote_context,
            self.expected_hash,
        )
    }
}

pub fn initiator_process(
    mode: KKTMode,
    ciphersuite: &Ciphersuite,
    own_encapsulation_key: Option<&EncapsulationKey>,
) -> Result<(KKTContext, KKTFrame), KKTError> {
    let context = KKTContext::new(KKTRole::Initiator, mode, ciphersuite);

    let body: &[u8] = match mode {
        KKTMode::OneWay => &[],
        KKTMode::Mutual => match own_encapsulation_key {
            Some(encaps_key) => &encaps_key.encode(),

            // Missing key
            None => {
                return Err(KKTError::FunctionInputError {
                    info: "KEM Key Not Provided",
                });
            }
        },
    };

    let frame = KKTFrame::new(&context, body)?;

    Ok((context, frame))
}

pub fn initiator_ingest_response(
    own_context: &mut KKTContext,
    remote_frame: &KKTFrame,
    remote_context: &KKTContext,
    expected_hash: &[u8],
) -> Result<(EncapsulationKey, bool), KKTError> {
    match remote_context.status() {
        KKTStatus::Ok | KKTStatus::UnverifiedKEMKey => {
            let received_encapsulation_key =
                EncapsulationKey::decode(own_context.ciphersuite().kem(), remote_frame.body_ref())?;

            match validate_encapsulation_key(
                own_context.ciphersuite().hash_function(),
                own_context.ciphersuite().hash_len(),
                remote_frame.body_ref(),
                expected_hash,
            ) {
                true => Ok((
                    received_encapsulation_key,
                    remote_context.status() != KKTStatus::UnverifiedKEMKey,
                )),

                // The key does not match the hash obtained from the directory
                false => Err(KKTError::KEMError {
                    info: "Hash of received encapsulation key does not match the value stored on the directory.",
                }),
            }
        }
        _ => Err(KKTError::ResponderFlaggedError {
            status: remote_context.status(),
        }),
    }
}
