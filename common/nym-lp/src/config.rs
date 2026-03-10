// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Configuration for LP protocol.
//!
//! LP security stack = KKT (key fetch) → PSQ (PQ PSK) → Noise (transport).
//! KEM algorithm selection affects only PSQ layer. Noise always uses X25519 DH.
//! Migration to PQ KEMs (MlKem768, XWing) requires only config change.

use nym_kkt::ciphersuite::KEM;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default PSK time-to-live (1 hour, matches psk.rs implementation).
pub const DEFAULT_PSK_TTL_SECS: u64 = 3600;

/// Configuration for LP protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpConfig {
    /// KEM algorithm for PSQ key encapsulation.
    /// Supported KEMs: MlKem768, McEliece
    #[serde(with = "kem_serde")]
    pub kem_algorithm: KEM,

    /// PSK time-to-live in seconds.
    pub psk_ttl_secs: u64,

    /// Enable KKT for authenticated key distribution.
    pub enable_kkt: bool,
}

impl Default for LpConfig {
    fn default() -> Self {
        Self {
            kem_algorithm: KEM::MlKem768,
            psk_ttl_secs: DEFAULT_PSK_TTL_SECS,
            enable_kkt: true,
        }
    }
}

impl LpConfig {
    /// Returns PSK TTL as Duration.
    pub fn psk_ttl(&self) -> Duration {
        Duration::from_secs(self.psk_ttl_secs)
    }
}

mod kem_serde {
    use nym_kkt::ciphersuite::KEM;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(kem: &KEM, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match kem {
            KEM::MlKem768 => "MlKem768",
            KEM::McEliece => "McEliece",
            KEM::X25519 => return Err(serde::ser::Error::custom("Unsupported KEM: X25519")),
            KEM::XWing => return Err(serde::ser::Error::custom("Unsupported KEM: XWing")),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<KEM, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "MlKem768" => Ok(KEM::MlKem768),
            "McEliece" => Ok(KEM::McEliece),
            "X25519" => Err(serde::de::Error::custom("Unsupported KEM: X25519")),
            "XWing" => Err(serde::de::Error::custom("Unsupported KEM: XWing")),
            _ => Err(serde::de::Error::custom(format!("Unknown KEM: {}", s))),
        }
    }
}
