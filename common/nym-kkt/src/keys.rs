// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::KKTError;
use libcrux_ml_kem::mlkem768::{MlKem768KeyPair, MlKem768PrivateKey, MlKem768PublicKey};
use libcrux_psq::classic_mceliece;
use libcrux_psq::handshake::types::PQEncapsulationKey;
use nym_kkt_ciphersuite::{KEM, mceliece};
use std::fmt::{Debug, Formatter};

/// Wrapper around keys used for the KEM exchange
pub struct KEMKeys {
    mc_eliece: classic_mceliece::KeyPair,
    ml_kem768: MlKem768KeyPair,
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
        Self {
            mc_eliece,
            ml_kem768,
        }
    }

    pub fn encoded_encapsulation_key(&self, kem: KEM) -> Option<&[u8]> {
        match kem {
            KEM::McEliece => Some(self.mc_eliece.pk.as_ref()),
            KEM::MlKem768 => Some(self.ml_kem768.pk()),
            _ => None,
        }
    }

    pub fn mc_eliece_encapsulation_key(&self) -> &classic_mceliece::PublicKey {
        &self.mc_eliece.pk
    }

    pub fn ml_kem768_encapsulation_key(&self) -> &MlKem768PublicKey {
        self.ml_kem768.public_key()
    }

    pub fn mc_eliece_decapsulation_key(&self) -> &classic_mceliece::SecretKey {
        &self.mc_eliece.sk
    }

    pub fn ml_kem768_decapsulation_key(&self) -> &MlKem768PrivateKey {
        self.ml_kem768.private_key()
    }
}

pub enum EncapsulationKey {
    McEliece(classic_mceliece::PublicKey),
    MlKem768(MlKem768PublicKey),
}

impl EncapsulationKey {
    pub fn kem(&self) -> KEM {
        match self {
            EncapsulationKey::McEliece(_) => KEM::McEliece,
            EncapsulationKey::MlKem768(_) => KEM::MlKem768,
        }
    }

    pub fn as_pq_encapsulation_key(&self) -> PQEncapsulationKey {
        match self {
            EncapsulationKey::McEliece(pk) => PQEncapsulationKey::CMC(pk),
            EncapsulationKey::MlKem768(pk) => PQEncapsulationKey::MlKem(pk),
        }
    }

    pub fn try_from_bytes(bytes: Vec<u8>, kem: KEM) -> Result<EncapsulationKey, KKTError> {
        match kem {
            KEM::MlKem768 => Ok(EncapsulationKey::MlKem768(
                MlKem768PublicKey::try_from(bytes.as_slice()).unwrap(),
            )),
            KEM::McEliece => {
                let boxed_array: Box<[u8; mceliece::PUBLIC_KEY_LENGTH]> = bytes
                    .into_boxed_slice()
                    .try_into()
                    .map_err(|_| KKTError::KEMError {
                        info: "mceliece key of invalid length",
                    })?;

                Ok(EncapsulationKey::McEliece(
                    classic_mceliece::PublicKey::try_from(boxed_array).unwrap(),
                ))
            }
        }
    }
}

impl Debug for EncapsulationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EncapsulationKey::McEliece(_) => write!(f, "McEliece"),
            EncapsulationKey::MlKem768(_) => write!(f, "MlKem768"),
        }
    }
}
