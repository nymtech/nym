// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LewesProtocol {
    /// Helper field that specifies whether the LP listener(s) is enabled on this node.
    /// It is directly controlled by the node's role (i.e. it is enabled if it supports 'entry' mode)
    pub enabled: bool,

    /// LP TCP control address (default: 41264) for establishing LP sessions
    pub control_port: u16,

    /// LP UDP data address (default: 51264) for Sphinx packets wrapped in LP
    pub data_port: u16,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    /// LP public key
    pub x25519: x25519::PublicKey,

    /// Digests of the KEM keys available to this node alongside hashing algorithms used
    /// for their computation.
    /// note: digests are hex encoded
    pub kem_keys: HashMap<LPKEM, HashMap<LPHashFunction, String>>,

    /// Digests of the signing keys available to this node alongside hashing algorithms used
    /// for their computation.
    /// note: digests are hex encoded
    pub signing_keys: HashMap<LPSignatureScheme, HashMap<LPHashFunction, String>>,
}

impl LewesProtocol {
    pub fn new(
        enabled: bool,
        control_port: u16,
        data_port: u16,
        x25519: x25519::PublicKey,
        kem_keys: HashMap<LPKEM, HashMap<LPHashFunction, String>>,
        signing_keys: HashMap<LPSignatureScheme, HashMap<LPHashFunction, String>>,
    ) -> Self {
        LewesProtocol {
            enabled,
            control_port,
            data_port,
            x25519,
            kem_keys,
            signing_keys,
        }
    }
}

// explicitly redefine available HashFunctions and KEMs so that we would not
// accidentally remove some type and thus break backwards compatibility at deserialisation level
// (the only thing that should break at that point would be conversion into proper nym-kkt types
// which would return a concrete error variant)

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Display,
    EnumString,
)]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LPKEM {
    MlKem768,
    XWing,
    X25519,
    McEliece,
}

impl From<LPKEM> for nym_kkt_ciphersuite::KEM {
    fn from(lpkem: LPKEM) -> Self {
        match lpkem {
            LPKEM::MlKem768 => nym_kkt_ciphersuite::KEM::MlKem768,
            LPKEM::XWing => nym_kkt_ciphersuite::KEM::XWing,
            LPKEM::X25519 => nym_kkt_ciphersuite::KEM::X25519,
            LPKEM::McEliece => nym_kkt_ciphersuite::KEM::McEliece,
        }
    }
}

impl From<nym_kkt_ciphersuite::KEM> for LPKEM {
    fn from(kem: nym_kkt_ciphersuite::KEM) -> Self {
        match kem {
            nym_kkt_ciphersuite::KEM::MlKem768 => LPKEM::MlKem768,
            nym_kkt_ciphersuite::KEM::XWing => LPKEM::XWing,
            nym_kkt_ciphersuite::KEM::X25519 => LPKEM::X25519,
            nym_kkt_ciphersuite::KEM::McEliece => LPKEM::McEliece,
        }
    }
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Display,
    EnumString,
    EnumIter,
)]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LPHashFunction {
    Blake3,
    Shake128,
    Shake256,
    Sha256,
}

impl From<LPHashFunction> for nym_kkt_ciphersuite::HashFunction {
    fn from(lp_hash_fnction: LPHashFunction) -> Self {
        match lp_hash_fnction {
            LPHashFunction::Blake3 => nym_kkt_ciphersuite::HashFunction::Blake3,
            LPHashFunction::Shake128 => nym_kkt_ciphersuite::HashFunction::Shake128,
            LPHashFunction::Shake256 => nym_kkt_ciphersuite::HashFunction::Shake256,
            LPHashFunction::Sha256 => nym_kkt_ciphersuite::HashFunction::SHA256,
        }
    }
}

impl From<nym_kkt_ciphersuite::HashFunction> for LPHashFunction {
    fn from(kem: nym_kkt_ciphersuite::HashFunction) -> Self {
        match kem {
            nym_kkt_ciphersuite::HashFunction::Blake3 => LPHashFunction::Blake3,
            nym_kkt_ciphersuite::HashFunction::Shake128 => LPHashFunction::Shake128,
            nym_kkt_ciphersuite::HashFunction::Shake256 => LPHashFunction::Shake256,
            nym_kkt_ciphersuite::HashFunction::SHA256 => LPHashFunction::Sha256,
        }
    }
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Display,
    EnumString,
    EnumIter,
)]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LPSignatureScheme {
    Ed25519,
}

impl From<LPSignatureScheme> for nym_kkt_ciphersuite::SignatureScheme {
    fn from(lp_hash_fnction: LPSignatureScheme) -> Self {
        match lp_hash_fnction {
            LPSignatureScheme::Ed25519 => nym_kkt_ciphersuite::SignatureScheme::Ed25519,
        }
    }
}

impl From<nym_kkt_ciphersuite::SignatureScheme> for LPSignatureScheme {
    fn from(kem: nym_kkt_ciphersuite::SignatureScheme) -> Self {
        match kem {
            nym_kkt_ciphersuite::SignatureScheme::Ed25519 => LPSignatureScheme::Ed25519,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_kkt_ciphersuite::SignatureScheme;

    #[test]
    fn kem_display() {
        assert_eq!(LPKEM::MlKem768.to_string(), "mlkem768");
        assert_eq!(LPKEM::XWing.to_string(), "xwing");
        assert_eq!(LPKEM::X25519.to_string(), "x25519");
        assert_eq!(LPKEM::McEliece.to_string(), "mceliece");
    }

    #[test]
    fn hash_function_display() {
        assert_eq!(LPHashFunction::Blake3.to_string(), "blake3");
        assert_eq!(LPHashFunction::Shake128.to_string(), "shake128");
        assert_eq!(LPHashFunction::Shake256.to_string(), "shake256");
        assert_eq!(LPHashFunction::Sha256.to_string(), "sha256");
    }

    #[test]
    fn signature_scheme_display() {
        assert_eq!(SignatureScheme::Ed25519.to_string(), "ed25519");
    }
}
