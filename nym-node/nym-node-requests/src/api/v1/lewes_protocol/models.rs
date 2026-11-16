// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api::{SignedData, SignedLewesProtocol};
use crate::error::Error;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_dh_public_key;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt_ciphersuite::KEMKeysDigests;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

    #[serde(with = "bs58_dh_public_key")]
    #[schemars(with = "String")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    /// LP public key
    pub x25519: x25519::DHPublicKey,

    /// Digests of the KEM keys available to this node alongside hashing algorithms used
    /// for their computation.
    /// note: digests are hex encoded
    pub kem_keys: BTreeMap<LPKEM, BTreeMap<LPHashFunction, String>>,
}

impl LewesProtocol {
    pub fn new(
        control_port: u16,
        data_port: u16,
        x25519: x25519::DHPublicKey,
        kem_key_digests: KEMKeysDigests,
    ) -> Self {
        let kem_keys = kem_key_digests
            .into_iter()
            .map(|(kem, digests)| {
                let digests = digests
                    .into_iter()
                    .map(|(hash_function, digest)| (hash_function.into(), hex::encode(digest)))
                    .collect();
                (kem.into(), digests)
            })
            .collect();

        LewesProtocol {
            enabled: true,
            control_port,
            data_port,
            x25519,
            kem_keys,
        }
    }
}

/// Rebuilds the signed HTTP API model a node published to the directory, byte-exact with what
/// it signed, so the legacy signature verifies.
impl TryFrom<nym_directory_types::LewesProtocolDetails> for SignedLewesProtocol {
    type Error = Error;

    fn try_from(details: nym_directory_types::LewesProtocolDetails) -> Result<Self, Self::Error> {
        let x25519 = x25519::PublicKey::from(details.x25519_public_key_bytes()?).into();
        let control_port = details.get_control_port();
        let data_port = details.get_data_port();
        let signature = ed25519::Signature::from_bytes(&details.legacy_http_api_signature)?;

        // the directory names KEMs and hash functions by their `nym_kkt_ciphersuite` display form
        let kem_key_digests: KEMKeysDigests = details
            .kem_key_digests
            .into_iter()
            .map(|(kem, digests)| {
                let digests = digests
                    .digests
                    .into_iter()
                    .map(|(hash_function, digest)| Ok((hash_function.parse()?, digest)))
                    .collect::<Result<_, Error>>()?;
                Ok((kem.parse()?, digests))
            })
            .collect::<Result<_, Error>>()?;

        Ok(SignedData {
            data: LewesProtocol::new(control_port, data_port, x25519, kem_key_digests),
            signature,
        })
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
    Ord,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LPKEM {
    #[serde(alias = "ml_kem768")]
    MlKem768,

    #[serde(alias = "mc_eliece")]
    McEliece,
}

impl From<LPKEM> for nym_kkt_ciphersuite::KEM {
    fn from(lpkem: LPKEM) -> Self {
        match lpkem {
            LPKEM::MlKem768 => nym_kkt_ciphersuite::KEM::MlKem768,
            LPKEM::McEliece => nym_kkt_ciphersuite::KEM::McEliece,
        }
    }
}

impl From<nym_kkt_ciphersuite::KEM> for LPKEM {
    fn from(kem: nym_kkt_ciphersuite::KEM) -> Self {
        match kem {
            nym_kkt_ciphersuite::KEM::MlKem768 => LPKEM::MlKem768,
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
    Ord,
)]
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "lowercase")]
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
