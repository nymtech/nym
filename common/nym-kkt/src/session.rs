use nym_crypto::asymmetric::ed25519::{self, Signature};
use rand::{CryptoRng, RngCore};

use crate::{
    ciphersuite::{Ciphersuite, EncapsulationKey},
    context::{KKTContext, KKTMode, KKTRole, KKTStatus},
    error::KKTError,
    frame::{KKTFrame, KKT_SESSION_ID_LEN},
    key_utils::validate_encapsulation_key,
};

pub fn initiator_process<'a, R>(
    rng: &mut R,
    mode: KKTMode,
    ciphersuite: Ciphersuite,
    signing_key: &ed25519::PrivateKey,
    own_encapsulation_key: Option<&EncapsulationKey<'a>>,
) -> Result<(KKTContext, KKTFrame), KKTError>
where
    R: CryptoRng + RngCore,
{
    let context = KKTContext::new(KKTRole::Initiator, mode, ciphersuite)?;

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
                })
            }
        },
    };

    let mut bytes_to_sign =
        Vec::with_capacity(context.full_message_len() - context.signature_len());
    bytes_to_sign.extend_from_slice(&context_bytes);
    bytes_to_sign.extend_from_slice(body);
    bytes_to_sign.extend_from_slice(&session_id);

    let signature = signing_key.sign(bytes_to_sign).to_bytes();

    Ok((
        context,
        KKTFrame::new(&context_bytes, body, &session_id, &signature),
    ))
}

pub fn anonymous_initiator_process<R>(
    rng: &mut R,
    ciphersuite: Ciphersuite,
) -> Result<(KKTContext, KKTFrame), KKTError>
where
    R: CryptoRng + RngCore,
{
    let context = KKTContext::new(KKTRole::AnonymousInitiator, KKTMode::OneWay, ciphersuite)?;
    let context_bytes = context.encode()?;

    let mut session_id = [0u8; KKT_SESSION_ID_LEN];
    rng.fill_bytes(&mut session_id);

    Ok((
        context,
        KKTFrame::new(&context_bytes, &[], &session_id, &[]),
    ))
}

pub fn initiator_ingest_response<'a>(
    own_context: &mut KKTContext,
    remote_verification_key: &ed25519::PublicKey,
    expected_hash: &[u8],
    message_bytes: &[u8],
) -> Result<EncapsulationKey<'a>, KKTError> {
    // sizes have to be correct
    let (frame, remote_context) = KKTFrame::from_bytes(message_bytes)?;

    check_compatibility(own_context, &remote_context)?;
    match remote_context.status() {
        KKTStatus::Ok => {
            let mut bytes_to_verify: Vec<u8> = Vec::with_capacity(
                remote_context.full_message_len() - remote_context.signature_len(),
            );
            bytes_to_verify.extend_from_slice(&remote_context.encode()?);
            bytes_to_verify.extend_from_slice(frame.body_ref());
            bytes_to_verify.extend_from_slice(frame.session_id_ref());

            match Signature::from_bytes(frame.signature_ref()) {
                Ok(sig) => match remote_verification_key.verify(bytes_to_verify, &sig) {
                    Ok(()) => {
                        let received_encapsulation_key = EncapsulationKey::decode(
                            own_context.ciphersuite().kem(),
                            frame.body_ref(),
                        )?;

                        match validate_encapsulation_key(
                    &own_context.ciphersuite().hash_function(),
                    own_context.ciphersuite().hash_len(),
                    frame.body_ref(),
                    expected_hash,
                ) {
                    true => Ok(received_encapsulation_key),

                    // The key does not match the hash obtained from the directory
                    false => Err(KKTError::KEMError { info: "Hash of received encapsulation key does not match the value stored on the directory." }),
                }
                    }
                    Err(_) => Err(KKTError::SigVerifError),
                },
                Err(_) => Err(KKTError::SigConstructorError),
            }
        }
        _ => Err(KKTError::ResponderFlaggedError {
            status: remote_context.status(),
        }),
    }
}

// todo: figure out how to handle errors using status codes

pub fn responder_ingest_message<'a>(
    remote_context: &KKTContext,
    remote_verification_key: Option<&ed25519::PublicKey>,
    expected_hash: Option<&[u8]>,
    remote_frame: &KKTFrame,
) -> Result<(KKTContext, Option<EncapsulationKey<'a>>), KKTError> {
    let own_context = remote_context.derive_responder_header()?;

    match remote_context.role() {
        KKTRole::AnonymousInitiator => Ok((own_context, None)),

        KKTRole::Initiator => {
            match remote_verification_key {
                Some(remote_verif_key) => {
                    let mut bytes_to_verify: Vec<u8> = Vec::with_capacity(
                        own_context.full_message_len() - own_context.signature_len(),
                    );
                    bytes_to_verify.extend_from_slice(remote_frame.context_ref());
                    bytes_to_verify.extend_from_slice(remote_frame.body_ref());
                    bytes_to_verify.extend_from_slice(remote_frame.session_id_ref());

                    match Signature::from_bytes(remote_frame.signature_ref()) {
                        Ok(sig) => match remote_verif_key.verify(bytes_to_verify, &sig) {
                            Ok(()) => {
                                // using own_context here because maybe for whatever reason we want to ignore the remote kem key
                                match own_context.mode() {
                                    KKTMode::OneWay => Ok((own_context, None)),
                                    KKTMode::Mutual => {
                                        match expected_hash {
    Some(expected_hash) =>{
      let received_encapsulation_key =
                    EncapsulationKey::decode(own_context.ciphersuite().kem(), remote_frame.body_ref())?;
                    if
                validate_encapsulation_key(
                    &own_context.ciphersuite().hash_function(),
                    own_context.ciphersuite().hash_len(),
                    remote_frame.body_ref(),
                    expected_hash,
                ){
                    Ok((own_context, Some(received_encapsulation_key)))
                }
                    // The key does not match the hash obtained from the directory
                  else {
                        Err(KKTError::KEMError { info: "Hash of received encapsulation key does not match the value stored on the directory." })
                    }
            }
            None => Err(KKTError::FunctionInputError { info: "Expected hash of the remote encapsulation key is not provided." }),
        }
                                    }
                                }
                            }
                            Err(_) => Err(KKTError::SigVerifError),
                        },
                        Err(_) => Err(KKTError::SigConstructorError),
                    }
                }
                None => Err(KKTError::FunctionInputError {
                    info: "Remote Signature Verification Key Not Provided",
                }),
            }
        }
        KKTRole::Responder => Err(KKTError::IncompatibilityError {
            info: "Responder received a request from another responder.",
        }),
    }
}

pub fn responder_process<'a>(
    own_context: &mut KKTContext,
    session_id: &[u8],
    signing_key: &ed25519::PrivateKey,
    encapsulation_key: &EncapsulationKey<'a>,
) -> Result<KKTFrame, KKTError> {
    let body = encapsulation_key.encode();

    let context_bytes = own_context.encode()?;

    let mut bytes_to_sign =
        Vec::with_capacity(own_context.full_message_len() - own_context.signature_len());
    bytes_to_sign.extend_from_slice(&own_context.encode()?);
    bytes_to_sign.extend_from_slice(&body);
    bytes_to_sign.extend_from_slice(session_id);

    let signature = signing_key.sign(bytes_to_sign).to_bytes();

    Ok(KKTFrame::new(&context_bytes, &body, session_id, &signature))
}

fn check_compatibility(
    _own_context: &KKTContext,
    _remote_context: &KKTContext,
) -> Result<(), KKTError> {
    // todo: check ciphersuite/context compatibility
    Ok(())
}
