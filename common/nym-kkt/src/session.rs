use rand09::{CryptoRng, RngCore};

use crate::frame::KKTSessionId;
use crate::{
    ciphersuite::{Ciphersuite, EncapsulationKey},
    context::{KKTContext, KKTMode, KKTRole, KKTStatus},
    error::KKTError,
    frame::{KKT_SESSION_ID_LEN, KKTFrame},
    key_utils::validate_encapsulation_key,
};

pub fn initiator_process<R>(
    rng: &mut R,
    mode: KKTMode,
    ciphersuite: Ciphersuite,
    own_encapsulation_key: Option<&EncapsulationKey>,
) -> Result<(KKTContext, KKTFrame), KKTError>
where
    R: CryptoRng + RngCore,
{
    let context = KKTContext::new(KKTRole::Initiator, mode, ciphersuite);

    let context_bytes = context.encode()?;

    let mut session_id = [0; KKT_SESSION_ID_LEN];
    // Generate Session ID
    rng.fill_bytes(&mut session_id);

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

    Ok((context, KKTFrame::new(context_bytes, body, session_id)))
}

pub fn initiator_ingest_response(
    own_context: &mut KKTContext,
    remote_frame: &KKTFrame,
    remote_context: &KKTContext,
    expected_hash: &[u8],
) -> Result<EncapsulationKey, KKTError> {
    check_compatibility(own_context, remote_context)?;
    match remote_context.status() {
        KKTStatus::Ok => {
            let received_encapsulation_key =
                EncapsulationKey::decode(own_context.ciphersuite().kem(), remote_frame.body_ref())?;

            match validate_encapsulation_key(
                &own_context.ciphersuite().hash_function(),
                own_context.ciphersuite().hash_len(),
                remote_frame.body_ref(),
                expected_hash,
            ) {
                true => Ok(received_encapsulation_key),

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

// todo: figure out how to handle errors using status codes

pub fn responder_ingest_message(
    remote_context: &KKTContext,
    expected_hash: Option<&[u8]>,
    remote_frame: &KKTFrame,
) -> Result<(KKTContext, Option<EncapsulationKey>), KKTError> {
    let own_context = remote_context.derive_responder_header()?;

    match remote_context.role() {
        KKTRole::Initiator => {
            // using own_context here because maybe for whatever reason we want to ignore the remote kem key
            match own_context.mode() {
                KKTMode::OneWay => Ok((own_context, None)),
                KKTMode::Mutual => {
                    match expected_hash {
                        Some(expected_hash) => {
                            let received_encapsulation_key = EncapsulationKey::decode(
                                own_context.ciphersuite().kem(),
                                remote_frame.body_ref(),
                            )?;
                            if validate_encapsulation_key(
                                &own_context.ciphersuite().hash_function(),
                                own_context.ciphersuite().hash_len(),
                                remote_frame.body_ref(),
                                expected_hash,
                            ) {
                                Ok((own_context, Some(received_encapsulation_key)))
                            }
                            // The key does not match the hash obtained from the directory
                            else {
                                Err(KKTError::KEMError {
                                    info: "Hash of received encapsulation key does not match the value stored on the directory.",
                                })
                            }
                        }
                        None => Err(KKTError::FunctionInputError {
                            info: "Expected hash of the remote encapsulation key is not provided.",
                        }),
                    }
                }
            }
        }

        KKTRole::Responder => Err(KKTError::IncompatibilityError {
            info: "Responder received a request from another responder.",
        }),
    }
}

pub fn responder_process(
    own_context: &mut KKTContext,
    session_id: KKTSessionId,
    encapsulation_key: &EncapsulationKey,
) -> Result<KKTFrame, KKTError> {
    let body = encapsulation_key.encode();
    let context_bytes = own_context.encode()?;
    Ok(KKTFrame::new(context_bytes, &body, session_id))
}

fn check_compatibility(
    _own_context: &KKTContext,
    _remote_context: &KKTContext,
) -> Result<(), KKTError> {
    // todo: check ciphersuite/context compatibility
    Ok(())
}
