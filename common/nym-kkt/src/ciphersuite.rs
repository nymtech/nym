// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::KKTError;
use libcrux_kem::Algorithm;

use std::fmt::Debug;

pub use nym_kkt_ciphersuite::*;

pub enum EncapsulationKey<'a> {
    MlKem768(libcrux_kem::MlKem768PublicKey),
    XWing(libcrux_kem::PublicKey),
    X25519(libcrux_kem::PublicKey),
    McEliece(classic_mceliece_rust::PublicKey<'a>),
}
impl<'a> Debug for EncapsulationKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MlKem768(_) => f.debug_tuple("MlKem768").finish(),
            Self::XWing(_) => f.debug_tuple("XWing").finish(),
            Self::X25519(_) => f.debug_tuple("X25519").finish(),
            Self::McEliece(_) => f.debug_tuple("McEliece").finish(),
        }
    }
}
impl<'a> EncapsulationKey<'a> {
    pub(crate) fn decode(kem: KEM, bytes: &[u8]) -> Result<Self, KKTError> {
        match kem {
            KEM::McEliece => {
                if bytes.len() != classic_mceliece_rust::CRYPTO_PUBLICKEYBYTES {
                    Err(KKTError::KEMError {
                        info: "Received McEliece Encapsulation Key with Invalid Length",
                    })
                } else {
                    let mut public_key_bytes =
                        Box::new([0u8; classic_mceliece_rust::CRYPTO_PUBLICKEYBYTES]);
                    // Size must be correct due to KKTFrame::from_bytes(message_bytes)?
                    public_key_bytes.clone_from_slice(bytes);
                    Ok(EncapsulationKey::McEliece(
                        classic_mceliece_rust::PublicKey::from(public_key_bytes),
                    ))
                }
            }
            KEM::X25519 => Ok(EncapsulationKey::X25519(libcrux_kem::PublicKey::decode(
                map_kem_to_libcrux_kem(kem)?,
                bytes,
            )?)),
            KEM::MlKem768 => Ok(EncapsulationKey::MlKem768(
                libcrux_kem::MlKem768PublicKey::try_from(bytes).map_err(|_| {
                    KKTError::DecodingError {
                        info: "MlKem Encapsulation Key Decoding Failure",
                    }
                })?,
            )),
            KEM::XWing => Ok(EncapsulationKey::XWing(libcrux_kem::PublicKey::decode(
                map_kem_to_libcrux_kem(kem)?,
                bytes,
            )?)),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            EncapsulationKey::XWing(public_key) | EncapsulationKey::X25519(public_key) => {
                public_key.encode()
            }
            EncapsulationKey::McEliece(public_key) => Vec::from(public_key.as_array()),
            EncapsulationKey::MlKem768(public_key) => Vec::from(public_key.as_slice()),
        }
    }
}
pub enum DecapsulationKey<'a> {
    MlKem768(libcrux_kem::MlKem768PrivateKey),
    XWing(libcrux_kem::PrivateKey),
    X25519(libcrux_kem::PrivateKey),
    McEliece(classic_mceliece_rust::SecretKey<'a>),
}
impl<'a> Debug for DecapsulationKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MlKem768(_) => f.debug_tuple("MlKem768").finish(),
            Self::XWing(_) => f.debug_tuple("XWing").finish(),
            Self::X25519(_) => f.debug_tuple("X25519").finish(),
            Self::McEliece(_) => f.debug_tuple("McEliece").finish(),
        }
    }
}

pub const fn map_kem_to_libcrux_kem(kem: KEM) -> Result<Algorithm, KKTError> {
    match kem {
        KEM::MlKem768 => Ok(Algorithm::MlKem768),
        KEM::XWing => Ok(Algorithm::XWingKemDraft06),
        KEM::X25519 => Ok(Algorithm::X25519),
        KEM::McEliece => Err(KKTError::KEMMapping {
            info: "attempted to map McEliece KEM to libcrux_kem",
        }),
    }
}
