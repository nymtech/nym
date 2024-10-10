// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde_wasm_bindgen::Error;
use thiserror::Error;
use wasm_utils::wasm_error;

#[derive(Debug, Error)]
pub enum VpnApiLibError {
    #[error("{0}")]
    Json(String),

    #[error("[ecash] cryptographic failure: {source}")]
    EcashFailure {
        #[from]
        source: nym_compact_ecash::CompactEcashError,
    },

    #[error("provided invalid ticket type")]
    MalformedTicketType,

    #[error("the provided shares and issuers are not from the same epoch! {shares} and {issuers}")]
    InconsistentEpochId { shares: u64, issuers: u64 },

    #[error("failed to recover ed25519 private key from its base58 representation")]
    MalformedEd25519Key,
}

wasm_error!(VpnApiLibError);

impl From<serde_wasm_bindgen::Error> for VpnApiLibError {
    fn from(value: Error) -> Self {
        VpnApiLibError::Json(value.to_string())
    }
}
