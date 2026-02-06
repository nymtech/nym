// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::KKTError;
use libcrux_kem::Algorithm;
use libcrux_ml_kem::mlkem768::MlKem768KeyPair;
pub use nym_kkt_ciphersuite::*;
use std::fmt::Debug;

pub enum EncapsulationKey {
    MlKem768(libcrux_kem::MlKem768PublicKey),
    XWing(libcrux_kem::PublicKey),
    X25519(libcrux_kem::PublicKey),
    McEliece(libcrux_psq::classic_mceliece::PublicKey),
}

impl EncapsulationKey {
    pub fn kem(&self) -> KEM {
        match self {
            EncapsulationKey::MlKem768(_) => KEM::MlKem768,
            EncapsulationKey::XWing(_) => KEM::XWing,
            EncapsulationKey::X25519(_) => KEM::X25519,
            EncapsulationKey::McEliece(_) => KEM::McEliece,
        }
    }
}

// impl Clone for EncapsulationKey {
//     fn clone(&self) -> Self {
//         match self {
//             Self::MlKem768(arg0) => Self::MlKem768(arg0.clone()),
//             Self::XWing(arg0) => Self::XWing(
//                 libcrux_kem::PublicKey::decode(Algorithm::XWingKemDraft06, &arg0.encode()).unwrap(),
//             ),
//             Self::X25519(arg0) => Self::X25519(
//                 libcrux_kem::PublicKey::decode(Algorithm::X25519, &arg0.encode()).unwrap(),
//             ),
//             Self::McEliece(arg0) => {
//                 let mut array = [0u8; mceliece::PUBLIC_KEY_LENGTH];
//                 array.clone_from_slice(arg0.as_ref());

//                 Self::McEliece(libcrux_psq::classic_mceliece::PublicKey::from(Box::new(
//                     array,
//                 )))
//             }
//         }
//     }
// }
impl Debug for EncapsulationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MlKem768(_) => f.debug_tuple("MlKem768").finish(),
            Self::XWing(_) => f.debug_tuple("XWing").finish(),
            Self::X25519(_) => f.debug_tuple("X25519").finish(),
            Self::McEliece(_) => f.debug_tuple("McEliece").finish(),
        }
    }
}
impl EncapsulationKey {
    pub(crate) fn decode(kem: KEM, bytes: &[u8]) -> Result<Self, KKTError> {
        match kem {
            KEM::McEliece => {
                if bytes.len() != mceliece::PUBLIC_KEY_LENGTH {
                    Err(KKTError::KEMError {
                        info: "Received McEliece Encapsulation Key with Invalid Length",
                    })
                } else {
                    let mut public_key_bytes = Box::new([0u8; mceliece::PUBLIC_KEY_LENGTH]);
                    // Size must be correct due to KKTFrame::from_bytes(message_bytes)?
                    public_key_bytes.clone_from_slice(bytes);
                    Ok(EncapsulationKey::McEliece(
                        libcrux_psq::classic_mceliece::PublicKey::from(public_key_bytes),
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
            EncapsulationKey::McEliece(public_key) => {
                let bytes_ref: &[u8] = public_key.as_ref();
                Vec::from(bytes_ref)
            }
            EncapsulationKey::MlKem768(public_key) => Vec::from(public_key.as_slice()),
        }
    }
}

pub enum DecapsulationKey {
    MlKem768(libcrux_kem::MlKem768PrivateKey),
    XWing(libcrux_kem::PrivateKey),
    X25519(libcrux_kem::PrivateKey),
    McEliece(libcrux_psq::classic_mceliece::SecretKey),
}

impl DecapsulationKey {
    pub fn kem(&self) -> KEM {
        match self {
            DecapsulationKey::MlKem768(_) => KEM::MlKem768,
            DecapsulationKey::XWing(_) => KEM::XWing,
            DecapsulationKey::X25519(_) => KEM::X25519,
            DecapsulationKey::McEliece(_) => KEM::McEliece,
        }
    }
}

impl Debug for DecapsulationKey {
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

pub enum KemKeyPair {
    MlKem768 {
        encapsulation_key: libcrux_kem::MlKem768PublicKey,
        decapsulation_key: libcrux_kem::MlKem768PrivateKey,
    },
    XWing {
        encapsulation_key: libcrux_kem::PublicKey,
        decapsulation_key: libcrux_kem::PrivateKey,
    },
    X25519 {
        encapsulation_key: libcrux_kem::PublicKey,
        decapsulation_key: libcrux_kem::PrivateKey,
    },
    McEliece {
        encapsulation_key: libcrux_psq::classic_mceliece::PublicKey,
        decapsulation_key: libcrux_psq::classic_mceliece::SecretKey,
    },
}

impl Debug for KemKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

// #[derive(Debug)]
// pub struct KemKeyPair {
//     encapsulation_key: EncapsulationKey,
//     decapsulation_key: DecapsulationKey,
// }

impl KemKeyPair {
    pub fn kem(&self) -> KEM {
        match self {
            KemKeyPair::MlKem768 { .. } => KEM::MlKem768,
            KemKeyPair::XWing { .. } => KEM::XWing,
            KemKeyPair::X25519 { .. } => KEM::X25519,
            KemKeyPair::McEliece { .. } => KEM::McEliece,
        }
    }

    // pub fn encapsulation_key(&self) -> &EncapsulationKey {
    //     &self.encapsulation_key
    // }
    //
    // pub fn decapsulation_key(&self) -> &DecapsulationKey {
    //     &self.decapsulation_key
    // }

    pub fn encoded_encapsulation_key(&self) -> Vec<u8> {
        match self {
            KemKeyPair::MlKem768 {
                encapsulation_key, ..
            } => encapsulation_key.as_slice().to_vec(),
            KemKeyPair::XWing {
                encapsulation_key, ..
            } => encapsulation_key.encode(),
            KemKeyPair::X25519 {
                encapsulation_key, ..
            } => encapsulation_key.encode(),
            KemKeyPair::McEliece {
                encapsulation_key, ..
            } => encapsulation_key.as_ref().to_vec(),
        }
    }
}

impl From<MlKem768KeyPair> for KemKeyPair {
    fn from(keypair: MlKem768KeyPair) -> Self {
        let (sk, pk) = keypair.into_parts();
        todo!()
        // KemKeyPair {
        //     encapsulation_key: EncapsulationKey::MlKem768(pk),
        //     decapsulation_key: DecapsulationKey::MlKem768(sk),
        // }
    }
}

impl From<libcrux_psq::classic_mceliece::KeyPair> for KemKeyPair {
    fn from(keypair: libcrux_psq::classic_mceliece::KeyPair) -> Self {
        todo!()
        // KemKeyPair {
        //     encapsulation_key: EncapsulationKey::McEliece(keypair.pk),
        //     decapsulation_key: DecapsulationKey::McEliece(keypair.sk),
        // }
    }
}
