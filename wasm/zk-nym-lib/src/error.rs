// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bandwidth_controller::error::BandwidthControllerError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;
use wasm_utils::wasm_error;

#[derive(Debug, Error)]
pub enum WasmCredentialClientError {
    #[error(transparent)]
    BandwidthControllerError {
        #[from]
        source: BandwidthControllerError,
    },

    #[error("the passed credential value had a value of zero")]
    ZeroCoinValue,

    #[error("failed to use credential storage: {source}")]
    StorageError {
        #[from]
        source: nym_credential_storage::error::StorageError,
    },

    #[error(transparent)]
    NyxdFailure {
        #[from]
        source: NyxdError,
    },

    #[error("no nyxd endpoints have been provided - we can't interact with the chain")]
    NoNyxdEndpoints,

    #[error("the provided nyxd endpoint is malformed: {source}")]
    MalformedNyxdEndpoint {
        #[from]
        source: UrlParseError,
    },

    // #[error("The provided deposit value was malformed: {source}")]
    // MalformedCoin { source: serde_wasm_bindgen::Error },
    #[error("The provided deposit value was malformed: {source}")]
    // annoyingly cosmwasm hasn't exposed CoinFromStrError directly
    // so we have to rely on the dynamic dispatch here
    MalformedCoin { source: Box<dyn std::error::Error> },

    // #[error("Coin parse error")]
    // CoinParseError,
    // #[error("State error")]
    // StateError,
    #[error("The provided mnemonic was malformed: {source}")]
    MalformedMnemonic {
        #[from]
        source: bip39::Error,
    },

    #[error("The ticket book cannot be retrieved from the credential store")]
    TicketbookCredentialStoreIsNone,
}

wasm_error!(WasmCredentialClientError);
