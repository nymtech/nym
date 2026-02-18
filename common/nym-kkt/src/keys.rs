// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::KKTError;
use libcrux_ml_kem::mlkem768::{MlKem768KeyPair, MlKem768PrivateKey, MlKem768PublicKey};
use libcrux_psq::classic_mceliece;
use libcrux_psq::handshake::types::PQEncapsulationKey;
use nym_kkt_ciphersuite::{KEM, mceliece};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Wrapper around keys used for the KEM exchange
/// with cheap clones thanks to Arc wrappers
pub struct KEMKeys {
    mc_eliece_pk: Arc<classic_mceliece::PublicKey>,
    mc_eliece_sk: Arc<classic_mceliece::SecretKey>,
    ml_kem768_pk: Arc<MlKem768PublicKey>,
    ml_kem768_sk: Arc<MlKem768PrivateKey>,
}

impl Debug for KEMKeys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KEMKeys")
            .field("mc_eliece", &"<redacted>")
            .field("ml_kem768", &"<redacted>")
            .finish()
    }
}

impl KEMKeys {
    pub fn new(mc_eliece: classic_mceliece::KeyPair, ml_kem768: MlKem768KeyPair) -> Self {
        let (ml_kem768_sk, ml_kem768_pk) = ml_kem768.into_parts();
        Self {
            mc_eliece_pk: Arc::new(mc_eliece.pk),
            mc_eliece_sk: Arc::new(mc_eliece.sk),
            ml_kem768_pk: Arc::new(ml_kem768_pk),
            ml_kem768_sk: Arc::new(ml_kem768_sk),
        }
    }

    pub fn encoded_encapsulation_key(&self, kem: KEM) -> Option<&[u8]> {
        match kem {
            KEM::McEliece => Some(self.mc_eliece_pk.as_ref().as_ref()),
            KEM::MlKem768 => Some(self.ml_kem768_pk.as_slice()),
            // _ => None,
        }
    }

    pub fn encapsulation_key(&self, kem: KEM) -> Option<EncapsulationKey> {
        match kem {
            KEM::McEliece => Some(EncapsulationKey::McEliece(self.mc_eliece_pk.clone())),
            KEM::MlKem768 => Some(EncapsulationKey::MlKem768(self.ml_kem768_pk.clone())),
            // _ => None,
        }
    }

    pub fn mc_eliece_encapsulation_key(&self) -> &classic_mceliece::PublicKey {
        &self.mc_eliece_pk
    }

    pub fn ml_kem768_encapsulation_key(&self) -> &MlKem768PublicKey {
        self.ml_kem768_pk.as_ref()
    }

    pub fn mc_eliece_decapsulation_key(&self) -> &classic_mceliece::SecretKey {
        &self.mc_eliece_sk
    }

    pub fn ml_kem768_decapsulation_key(&self) -> &MlKem768PrivateKey {
        &self.ml_kem768_sk
    }
}

#[derive(Clone)]
pub enum EncapsulationKey {
    McEliece(Arc<classic_mceliece::PublicKey>),
    MlKem768(Arc<MlKem768PublicKey>),
}

impl Debug for EncapsulationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EncapsulationKey::McEliece(_) => write!(f, "EncapsulationKey::McEliece"),
            EncapsulationKey::MlKem768(_) => write!(f, "EncapsulationKey::MlKem768"),
        }
    }
}

impl EncapsulationKey {
    pub fn kem(&self) -> KEM {
        match self {
            EncapsulationKey::McEliece(_) => KEM::McEliece,
            EncapsulationKey::MlKem768(_) => KEM::MlKem768,
        }
    }

    pub fn as_pq_encapsulation_key(&self) -> PQEncapsulationKey<'_> {
        match self {
            EncapsulationKey::McEliece(pk) => PQEncapsulationKey::CMC(pk),
            EncapsulationKey::MlKem768(pk) => PQEncapsulationKey::MlKem(pk),
        }
    }

    pub fn try_from_bytes(bytes: Vec<u8>, kem: KEM) -> Result<EncapsulationKey, KKTError> {
        match kem {
            KEM::MlKem768 => Ok(EncapsulationKey::MlKem768(Arc::new(
                MlKem768PublicKey::try_from(bytes.as_slice()).map_err(|_| KKTError::KEMError {
                    info: "mlkem768 key of invalid length",
                })?,
            ))),
            KEM::McEliece => {
                let boxed_array: Box<[u8; mceliece::PUBLIC_KEY_LENGTH]> = bytes
                    .into_boxed_slice()
                    .try_into()
                    .map_err(|_| KKTError::KEMError {
                        info: "mceliece key of invalid length",
                    })?;

                Ok(EncapsulationKey::McEliece(Arc::new(
                    classic_mceliece::PublicKey::from(boxed_array),
                )))
            }
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            EncapsulationKey::McEliece(k) => k.as_ref().as_ref(),
            EncapsulationKey::MlKem768(k) => k.as_ref().as_ref(),
        }
    }
}
