// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

    /// Digests of the KEM keys available to this node alongside hashing algorithms used
    /// for their computation.
    pub kem_keys: HashMap<LPKEM, HashMap<LPHashFunction, Vec<u8>>>,
}

impl LewesProtocol {
    pub fn new(enabled: bool, control_port: u16, data_port: u16) -> Self {
        LewesProtocol {
            enabled,
            control_port,
            data_port,
            kem_keys: Default::default(),
        }
    }

    pub fn with_kem_key_hashes(
        mut self,
        kem: LPKEM,
        hashes: HashMap<LPHashFunction, Vec<u8>>,
    ) -> Self {
        self.kem_keys.insert(kem, hashes);
        self
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
