// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::KKTCiphersuiteError;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::HashMap;
use std::fmt::Display;
use strum_macros::EnumIter;

pub mod error;

pub const DEFAULT_HASH_LEN: usize = 32;
const _: () = assert!(DEFAULT_HASH_LEN <= u8::MAX as usize);

pub const MINIMUM_SECURE_HASH_LEN: u8 = 16;
const _: () = assert!(MINIMUM_SECURE_HASH_LEN <= DEFAULT_HASH_LEN as u8);

pub const CIPHERSUITE_ENCODING_LEN: usize = 4;

// no point in importing curve libraries for well-defined constants
pub mod ed25519 {
    pub const SECRET_KEY_LENGTH: usize = 32;
    pub const PUBLIC_KEY_LENGTH: usize = 32;
    pub const SIGNATURE_LENGTH: usize = 64;
}

pub mod x25519 {
    pub const PUBLIC_KEY_LENGTH: usize = 32;
    pub const SECRET_KEY_LENGTH: usize = 32;
}

pub mod ml_kem768 {
    pub const PUBLIC_KEY_LENGTH: usize = 1184;
}

pub mod mceliece {
    pub const PUBLIC_KEY_LENGTH: usize = 524160;
    pub const SECRET_KEY_LENGTH: usize = 13608;
    pub const CIPHERTEXT_LENGTH: usize = 156;
}

pub mod xwing {
    use crate::{ml_kem768, x25519};

    pub const PUBLIC_KEY_LENGTH: usize = x25519::PUBLIC_KEY_LENGTH + ml_kem768::PUBLIC_KEY_LENGTH;
}

pub type KEMKeyDigests = HashMap<HashFunction, Vec<u8>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, IntoPrimitive, TryFromPrimitive, EnumIter)]
#[repr(u8)]
pub enum HashFunction {
    Blake3 = 0,
    SHAKE256 = 1,
    SHAKE128 = 2,
    SHA256 = 3,
}

impl HashFunction {
    #[cfg(feature = "digests")]
    pub fn digest<M: AsRef<[u8]>>(&self, data: M, output_length: usize) -> Vec<u8> {
        let mut out = vec![0u8; output_length];
        match self {
            HashFunction::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(data.as_ref());
                hasher.finalize_xof().fill(&mut out);
                hasher.reset();
            }
            HashFunction::SHAKE256 => libcrux_sha3::shake256_ema(&mut out, data.as_ref()),
            HashFunction::SHAKE128 => libcrux_sha3::shake128_ema(&mut out, data.as_ref()),
            HashFunction::SHA256 => libcrux_sha3::sha256_ema(&mut out, data.as_ref()),
        }

        out
    }
}

impl Display for HashFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            HashFunction::Blake3 => "blake3",
            HashFunction::SHAKE128 => "shake128",
            HashFunction::SHAKE256 => "shake256",
            HashFunction::SHA256 => "sha256",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, IntoPrimitive)]
#[repr(u8)]
pub enum HashLength {
    Default = 0,
    #[num_enum(catch_all)]
    Custom(u8),
}

impl Display for HashLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashLength::Default => DEFAULT_HASH_LEN.fmt(f),
            HashLength::Custom(custom_len) => custom_len.fmt(f),
        }
    }
}

impl HashLength {
    pub fn decode(raw: u8) -> Result<Self, KKTCiphersuiteError> {
        // check if we're using encoding for 'default' value
        if raw == u8::from(Self::Default) {
            return Ok(Self::Default);
        }
        // otherwise, we treat it as a custom length, and we have to validate its security
        let custom_len = raw;

        if custom_len < MINIMUM_SECURE_HASH_LEN {
            return Err(KKTCiphersuiteError::InsecureHashLen {
                requested: custom_len,
                minimum: MINIMUM_SECURE_HASH_LEN,
            });
        }
        Ok(Self::Custom(custom_len))
    }
}

impl TryFrom<Option<u8>> for HashLength {
    type Error = KKTCiphersuiteError;

    fn try_from(value: Option<u8>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(Self::Default),
            Some(custom_len) => Self::decode(custom_len),
        }
    }
}

impl HashLength {
    pub const fn value(&self) -> usize {
        match self {
            HashLength::Default => DEFAULT_HASH_LEN,
            HashLength::Custom(custom_len) => *custom_len as usize,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum SignatureScheme {
    Ed25519 = 0,
}

impl SignatureScheme {
    pub const fn signing_key_length(&self) -> usize {
        match self {
            // 32 bytes
            SignatureScheme::Ed25519 => ed25519::SECRET_KEY_LENGTH,
        }
    }

    pub const fn verification_key_length(&self) -> usize {
        match self {
            // 32 bytes
            SignatureScheme::Ed25519 => ed25519::PUBLIC_KEY_LENGTH,
        }
    }

    pub const fn signature_length(&self) -> usize {
        match self {
            // 64 bytes
            SignatureScheme::Ed25519 => ed25519::SIGNATURE_LENGTH,
        }
    }
}

impl Display for SignatureScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SignatureScheme::Ed25519 => "ed25519",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum KEM {
    XWing = 0,
    MlKem768 = 1,
    McEliece = 2,
    X25519 = 255,
}

impl KEM {
    pub fn encapsulation_key_length(&self) -> usize {
        match self {
            KEM::MlKem768 => ml_kem768::PUBLIC_KEY_LENGTH,
            KEM::XWing => xwing::PUBLIC_KEY_LENGTH,
            KEM::X25519 => x25519::PUBLIC_KEY_LENGTH,
            KEM::McEliece => mceliece::PUBLIC_KEY_LENGTH,
        }
    }
}

impl Display for KEM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KEM::MlKem768 => "mlkem768",
            KEM::XWing => "xwing",
            KEM::X25519 => "x25519",
            KEM::McEliece => "mceliece",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ciphersuite {
    hash_function: HashFunction,
    signature_scheme: SignatureScheme,
    kem: KEM,
    hash_length: HashLength,
    encapsulation_key_length: usize,
    signing_key_length: usize,
    verification_key_length: usize,
    signature_length: usize,
}

impl Ciphersuite {
    pub fn new(
        kem: KEM,
        hash_function: HashFunction,
        signature_scheme: SignatureScheme,
        hash_length: HashLength,
    ) -> Self {
        Self {
            hash_function,
            signature_scheme,
            kem,
            hash_length,
            encapsulation_key_length: kem.encapsulation_key_length(),
            signing_key_length: signature_scheme.signing_key_length(),
            verification_key_length: signature_scheme.verification_key_length(),
            signature_length: signature_scheme.signature_length(),
        }
    }

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
        self.hash_length.value()
    }

    pub fn resolve_ciphersuite(
        kem: KEM,
        hash_function: HashFunction,
        signature_scheme: SignatureScheme,
        // This should be None 99.9999% of the time
        custom_hash_length: Option<u8>,
    ) -> Result<Self, KKTCiphersuiteError> {
        let hash_length = HashLength::try_from(custom_hash_length)?;

        Ok(Ciphersuite::new(
            kem,
            hash_function,
            signature_scheme,
            hash_length,
        ))
    }
    pub fn encode(&self) -> [u8; CIPHERSUITE_ENCODING_LEN] {
        // [kem, hash, hashlen, sig]
        [
            self.kem.into(),
            self.hash_function.into(),
            self.hash_length.into(),
            self.signature_scheme.into(),
        ]
    }

    pub fn decode(encoding: [u8; CIPHERSUITE_ENCODING_LEN]) -> Result<Self, KKTCiphersuiteError> {
        let raw_kem = encoding[0];
        let raw_hash_function = encoding[1];
        let hash_len = encoding[2];
        let raw_signature_scheme = encoding[3];

        let kem = KEM::try_from(raw_kem)
            .map_err(|_| KKTCiphersuiteError::UnknownKEMType { raw: raw_kem })?;
        let hash_function = HashFunction::try_from(raw_hash_function).map_err(|_| {
            KKTCiphersuiteError::UnknownHashFunctionType {
                raw: raw_hash_function,
            }
        })?;
        let hash_length = HashLength::decode(hash_len)?;
        let signature_scheme = SignatureScheme::try_from(raw_signature_scheme).map_err(|_| {
            KKTCiphersuiteError::UnknownSignatureSchemeType {
                raw: raw_signature_scheme,
            }
        })?;

        Ok(Self::new(kem, hash_function, signature_scheme, hash_length))
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
