// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::vpn_api_client::NymVpnApiClientError;
use thiserror::Error;
use wasm_utils::wasm_error;

#[derive(Debug, Error)]
pub enum ZkNymError {
    #[error("[coconut] cryptographic failure: {source}")]
    CoconutFailure {
        #[from]
        source: nym_coconut::CoconutError,
    },

    #[error("[ecash] cryptographic failure: {source}")]
    EcashFailure {
        #[from]
        source: nym_compact_ecash::CompactEcashError,
    },

    #[error("failed to contact the vpn api")]
    HttpClientFailure {
        #[from]
        source: NymVpnApiClientError,
    },
    #[error("the provided shares and issuers are not from the same epoch! {shares} and {issuers}")]
    InconsistentEpochId { shares: u64, issuers: u64 },

    #[error("the provided deposit amount is malformed")]
    InvalidDepositAmount,

    #[error("global parameters have already been set before")]
    GlobalParamsAlreadySet,

    #[error("no parameters were provided - they need to be provided either explicitly or a global ones need to be set")]
    NoParametersProvided,

    #[error("failed to recover ed25519 private key from its base58 representation: {0}")]
    MalformedEd25519Key(String),

    #[error("failed to recover x25519 private key from its base58 representation: {0}")]
    MalformedX25519Key(String),

    #[error("failed to recover the deposit transaction hash from its [hex] representation: {0}")]
    MalformedTransactionHash(String),
}

wasm_error!(ZkNymError);
