use std::collections::HashSet;

use crate::key_utils::validate_encapsulation_key;
use crate::{
    ciphersuite::EncapsulationKey,
    context::{KKTContext, KKTMode, KKTRole, KKTStatus},
    error::KKTError,
    frame::KKTFrame,
};
use libcrux_psq::handshake::types::DHKeyPair;
use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, KEM, SignatureScheme};
pub struct KKTResponder<'a> {
    x25519_keypair: &'a DHKeyPair,
    mlkem_encapsulation_key: Option<&'a EncapsulationKey>,
    mceliece_encapsulation_key: Option<&'a EncapsulationKey>,
    supported_hash_functions: HashSet<HashFunction>,
    supported_signature_schemes: HashSet<SignatureScheme>,
    supported_outer_protocol_versions: HashSet<u8>,
}
impl<'a> KKTResponder<'a> {
    pub fn new(
        x25519_keypair: &'a DHKeyPair,
        mlkem_encapsulation_key: Option<&'a EncapsulationKey>,
        mceliece_encapsulation_key: Option<&'a EncapsulationKey>,
        supported_hash_functions: &[HashFunction],
        supported_outer_protocol_versions: &[u8],
        supported_signature_schemes: &[SignatureScheme],
    ) -> Result<Self, KKTError> {
        let hash_functions: HashSet<HashFunction> =
            supported_hash_functions.iter().copied().collect();

        if hash_functions.is_empty() {
            Err(KKTError::FunctionInputError {
                info: "Did not provide a supported HashFunction when instaciating a KKTResponder",
            })
        } else {
            let signature_schemes: HashSet<SignatureScheme> =
                supported_signature_schemes.iter().copied().collect();

            if signature_schemes.is_empty() {
                Err(KKTError::FunctionInputError {
                    info: "Did not provide a supported SignatureScheme when instaciating a KKTResponder",
                })
            } else {
                let outer_protocol_versions: HashSet<u8> =
                    supported_outer_protocol_versions.iter().copied().collect();

                if outer_protocol_versions.is_empty() {
                    Err(KKTError::FunctionInputError {
                        info: "Did not provide a supported outer protocol version when instaciating a KKTResponder",
                    })
                } else {
                    match (mlkem_encapsulation_key, mceliece_encapsulation_key) {
                        (Some(mlkem_key), Some(mceliece_key)) => match (mlkem_key, mceliece_key) {
                            (EncapsulationKey::MlKem768(_), EncapsulationKey::McEliece(_)) => {
                                Ok(Self {
                                    x25519_keypair,
                                    mlkem_encapsulation_key,
                                    mceliece_encapsulation_key,
                                    supported_hash_functions: hash_functions,
                                    supported_signature_schemes: signature_schemes,
                                    supported_outer_protocol_versions: outer_protocol_versions,
                                })
                            }
                            (EncapsulationKey::MlKem768(_), _) => {
                                Err(KKTError::FunctionInputError {
                                    info: "Provided a non-MlKem768 encapsulation key as the MlKem768 key.",
                                })
                            }
                            (EncapsulationKey::McEliece(_), _) => {
                                Err(KKTError::FunctionInputError {
                                    info: "Provided a non-McEliece encapsulation key as the McEliece key.",
                                })
                            }
                            _ => Err(KKTError::FunctionInputError {
                                info: "Provided incompatible encapsulation keys.",
                            }),
                        },
                        (Some(mlkem_key), None) => match mlkem_key {
                            EncapsulationKey::MlKem768(_) => Ok(Self {
                                x25519_keypair,
                                mlkem_encapsulation_key,
                                mceliece_encapsulation_key: None,
                                supported_hash_functions: hash_functions,
                                supported_signature_schemes: signature_schemes,
                                supported_outer_protocol_versions: outer_protocol_versions,
                            }),
                            _ => Err(KKTError::FunctionInputError {
                                info: "Provided a non-MlKem768 encapsulation key as the MlKem768 key.",
                            }),
                        },
                        (None, Some(mceliece_key)) => match mceliece_key {
                            EncapsulationKey::McEliece(_) => Ok(Self {
                                x25519_keypair,
                                mlkem_encapsulation_key: None,
                                mceliece_encapsulation_key,
                                supported_hash_functions: hash_functions,
                                supported_signature_schemes: signature_schemes,
                                supported_outer_protocol_versions: outer_protocol_versions,
                            }),
                            _ => Err(KKTError::FunctionInputError {
                                info: "Provided a non-McEliece encapsulation key as the McEliece key.",
                            }),
                        },
                        (None, None) => Err(KKTError::FunctionInputError {
                            info: "Did not provide an encapsulation key when instanciating a KKTResponder.",
                        }),
                    }
                }
            }
        }
    }
    fn supported_protocol_versions(&self) -> Vec<u8> {
        self.supported_outer_protocol_versions
            .iter()
            .copied()
            .collect()
    }

    fn check_ciphersuite_compatiblity(
        &self,
        remote_ciphersuite: &Ciphersuite,
    ) -> Result<(), KKTError> {
        if !self
            .supported_hash_functions
            .contains(remote_ciphersuite.hash_function())
        {
            Err(KKTError::IncompatibilityError {
                info: "Unsupported HashFunction",
            })
        } else {
            if !self
                .supported_signature_schemes
                .contains(remote_ciphersuite.signature_scheme())
            {
                Err(KKTError::IncompatibilityError {
                    info: "Unsupported SignatureScheme",
                })
            } else {
                if match remote_ciphersuite.kem() {
                    KEM::MlKem768 => self.mlkem_encapsulation_key.is_some(),
                    KEM::McEliece => self.mceliece_encapsulation_key.is_some(),
                    _ => false,
                } {
                    Ok(())
                } else {
                    Err(KKTError::IncompatibilityError {
                        info: "Unsupported KEM",
                    })
                }
            }
        }
    }

    // When this function fails, we do that silently (i.e. we dont generate a response to the initiator).

    pub fn process_request(
        &self,
        request_bytes: &[u8],
    ) -> Result<(Vec<u8>, Option<EncapsulationKey>), KKTError> {
        let (mut carrier, remote_frame, remote_context) = KKTFrame::decrypt_initiator_frame(
            self.x25519_keypair,
            request_bytes,
            &self.supported_protocol_versions(),
        )?;

        self.check_ciphersuite_compatiblity(remote_context.ciphersuite())?;

        let (local_context, remote_encapsulation_key) = match remote_context.mode() {
            KKTMode::OneWay => responder_ingest_message(&remote_context, None, &remote_frame)?,
            KKTMode::Mutual => {
                // So we can either fetch the remote hash here using some async call to the directory,
                // which might make registration hang or accept the sent key then verify later.

                // If we choose to not accept, the response's status will be KKTStatus::UnverifiedKEMKey.
                // The response would still contain the responder's encapsulation key.
                responder_ingest_message(&remote_context, None, &remote_frame)?
            }
        };

        let frame = if local_context.ciphersuite().kem() == &KEM::MlKem768 {
            KKTFrame::new(
                &local_context,
                // SAFETY: the self.check_ciphersuite_compatibility call above guarantees that we will have a key in the right place
                #[allow(clippy::unwrap_used)]
                &self.mlkem_encapsulation_key.unwrap().encode(),
            )?
        } else {
            KKTFrame::new(
                &local_context,
                // SAFETY: the self.check_ciphersuite_compatibility call above guarantees that we will have a key in the right place
                #[allow(clippy::unwrap_used)]
                &self.mceliece_encapsulation_key.unwrap().encode(),
            )?
        };

        // encryption - responder frame
        let response_bytes = carrier.encrypt(&frame.to_bytes())?;
        Ok((response_bytes, remote_encapsulation_key))
    }
}

pub fn responder_ingest_message(
    remote_context: &KKTContext,
    expected_hash: Option<&[u8]>,
    remote_frame: &KKTFrame,
) -> Result<(KKTContext, Option<EncapsulationKey>), KKTError> {
    let mut own_context = remote_context.derive_responder_header()?;

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
                                own_context.ciphersuite().hash_function(),
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
                        None => {
                            own_context.update_status(KKTStatus::UnverifiedKEMKey);
                            // we don't store an unverified key
                            // changing the status notifies the initiator that we didn't

                            // we could still keep it here and then verify later...
                            // let received_encapsulation_key = EncapsulationKey::decode(
                            //     own_context.ciphersuite().kem(),
                            //     remote_frame.body_ref(),
                            // )?;
                            //  Ok((own_context, Some(received_encapsulation_key)))
                            //

                            Ok((own_context, None))
                        }
                    }
                }
            }
        }

        KKTRole::Responder => Err(KKTError::IncompatibilityError {
            info: "Responder received a request from another responder.",
        }),
    }
}
