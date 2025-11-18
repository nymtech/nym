// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use libcrux_kem::{Algorithm, MlKem768PublicKey};
use nym_crypto::asymmetric::ed25519;

use crate::error::KKTError;

pub const HASH_LEN_256: u8 = 32;
pub const CIPHERSUITE_ENCODING_LEN: usize = 4;

pub const CURVE25519_KEY_LEN: usize = 32;

#[derive(Clone, Copy, Debug)]
pub enum HashFunction {
    Blake3,
    SHAKE128,
    SHAKE256,
    SHA256,
}
impl Display for HashFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            HashFunction::Blake3 => "Blake3",
            HashFunction::SHAKE128 => "SHAKE128",
            HashFunction::SHAKE256 => "SHAKE256",
            HashFunction::SHA256 => "SHA256",
        })
    }
}

pub enum EncapsulationKey<'a> {
    MlKem768(libcrux_kem::PublicKey),
    XWing(libcrux_kem::PublicKey),
    X25519(libcrux_kem::PublicKey),
    McEliece(classic_mceliece_rust::PublicKey<'a>),
}

pub enum DecapsulationKey<'a> {
    MlKem768(libcrux_kem::PrivateKey),
    XWing(libcrux_kem::PrivateKey),
    X25519(libcrux_kem::PrivateKey),
    McEliece(classic_mceliece_rust::SecretKey<'a>),
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
                map_kem_to_libcrux_kem(kem),
                bytes,
            )?)),
            KEM::MlKem768 => Ok(EncapsulationKey::MlKem768(libcrux_kem::PublicKey::decode(
                map_kem_to_libcrux_kem(kem),
                bytes,
            )?)),
            KEM::XWing => Ok(EncapsulationKey::XWing(libcrux_kem::PublicKey::decode(
                map_kem_to_libcrux_kem(kem),
                bytes,
            )?)),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            EncapsulationKey::XWing(public_key)
            | EncapsulationKey::MlKem768(public_key)
            | EncapsulationKey::X25519(public_key) => public_key.encode(),
            EncapsulationKey::McEliece(public_key) => Vec::from(public_key.as_array()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SignatureScheme {
    Ed25519,
}
impl Display for SignatureScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SignatureScheme::Ed25519 => "Ed25519",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum KEM {
    MlKem768,
    XWing,
    X25519,
    McEliece,
}

impl Display for KEM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KEM::MlKem768 => "MlKem768",
            KEM::XWing => "XWing",
            KEM::X25519 => "x25519",
            KEM::McEliece => "McEliece",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ciphersuite {
    hash_function: HashFunction,
    signature_scheme: SignatureScheme,
    kem: KEM,
    hash_length: u8,
    encapsulation_key_length: usize,
    signing_key_length: usize,
    verification_key_length: usize,
    signature_length: usize,
}

impl Ciphersuite {
    pub fn kem_key_len(&self) -> usize {
        self.encapsulation_key_length
    }

    pub fn signature_len(&self) -> usize {
        self.signature_length
    }
    pub fn signing_key_len(&self) -> usize {
        self.signing_key_length
    }
    pub fn verification_key_len(&self) -> usize {
        self.verification_key_length
    }
    pub fn hash_function(&self) -> HashFunction {
        self.hash_function
    }
    pub fn kem(&self) -> KEM {
        self.kem
    }
    pub fn signature_scheme(&self) -> SignatureScheme {
        self.signature_scheme
    }
    pub fn hash_len(&self) -> usize {
        self.hash_length as usize
    }

    pub fn default() -> Self {
        Self::resolve_ciphersuite(
            KEM::XWing,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            None,
        )
        .unwrap()
    }

    pub fn resolve_ciphersuite(
        kem: KEM,
        hash_function: HashFunction,
        signature_scheme: SignatureScheme,
        // This should be None 99.9999% of the time
        custom_hash_length: Option<u8>,
    ) -> Result<Self, KKTError> {
        let hash_len = match custom_hash_length {
            Some(l) => {
                if l < 16 {
                    return Err(KKTError::InsecureHashLen);
                } else {
                    l
                }
            }
            None => HASH_LEN_256,
        };
        Ok(Self {
            hash_function,
            signature_scheme,
            kem,
            hash_length: hash_len,
            encapsulation_key_length: match kem {
                // 1184 bytes
                KEM::MlKem768 => MlKem768PublicKey::len(),
                // 1216 bytes = 1184 + 32
                KEM::XWing => MlKem768PublicKey::len() + CURVE25519_KEY_LEN,
                // 32 bytes
                KEM::X25519 => CURVE25519_KEY_LEN,
                // 524160 bytes
                KEM::McEliece => classic_mceliece_rust::CRYPTO_PUBLICKEYBYTES,
            },
            signing_key_length: match signature_scheme {
                // 32 bytes
                SignatureScheme::Ed25519 => ed25519::SECRET_KEY_LENGTH,
            },
            verification_key_length: match signature_scheme {
                // 32 bytes
                SignatureScheme::Ed25519 => ed25519::PUBLIC_KEY_LENGTH,
            },
            signature_length: match signature_scheme {
                // 64 bytes
                SignatureScheme::Ed25519 => ed25519::SIGNATURE_LENGTH,
            },
        })
    }
    pub fn encode(&self) -> [u8; 4] {
        // [kem, hash, hashlen, sig]
        [
            match self.kem {
                KEM::XWing => 0,
                KEM::MlKem768 => 1,
                KEM::McEliece => 2,
                KEM::X25519 => 255,
            },
            match self.hash_function {
                HashFunction::Blake3 => 0,
                HashFunction::SHAKE256 => 1,
                HashFunction::SHAKE128 => 2,
                HashFunction::SHA256 => 3,
            },
            match self.hash_length {
                HASH_LEN_256 => 0,
                _ => self.hash_length,
            },
            match self.signature_scheme {
                SignatureScheme::Ed25519 => 0,
            },
        ]
    }
    pub fn decode(encoding: &[u8]) -> Result<Self, KKTError> {
        if encoding.len() == 4 {
            let kem = match encoding[0] {
                0 => KEM::XWing,
                1 => KEM::MlKem768,
                2 => KEM::McEliece,
                255 => KEM::X25519,
                _ => {
                    return Err(KKTError::CiphersuiteDecodingError {
                        info: format!("Undefined KEM: {}", encoding[0]),
                    })
                }
            };
            let hash_function = match encoding[1] {
                0 => HashFunction::Blake3,
                1 => HashFunction::SHAKE256,
                2 => HashFunction::SHAKE128,
                3 => HashFunction::SHA256,
                _ => {
                    return Err(KKTError::CiphersuiteDecodingError {
                        info: format!("Undefined Hash Function: {}", encoding[1]),
                    })
                }
            };

            let custom_hash_length = match encoding[2] {
                0 => None,
                _ => Some(encoding[2]),
            };

            let signature_scheme = match encoding[3] {
                0 => SignatureScheme::Ed25519,
                _ => {
                    return Err(KKTError::CiphersuiteDecodingError {
                        info: format!("Undefined Signature Scheme: {}", encoding[3]),
                    })
                }
            };

            Self::resolve_ciphersuite(kem, hash_function, signature_scheme, custom_hash_length)
        } else {
            Err(KKTError::CiphersuiteDecodingError {
                info: format!(
                    "Incorrect Encoding Length: actual: {} != expected: {}",
                    encoding.len(),
                    CIPHERSUITE_ENCODING_LEN
                ),
            })
        }
    }
}

impl Display for Ciphersuite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            &format!(
                "{}_{}({})_{}",
                self.kem, self.hash_function, self.hash_length, self.signature_scheme
            )
            .to_ascii_lowercase(),
        )
    }
}

pub const fn map_kem_to_libcrux_kem(kem: KEM) -> Algorithm {
    match kem {
        KEM::MlKem768 => Algorithm::MlKem768,
        KEM::XWing => Algorithm::XWingKemDraft06,
        KEM::X25519 => Algorithm::X25519,
        _ => unreachable!(),
    }
}
