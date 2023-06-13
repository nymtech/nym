// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_utils::simple_js_error;
use wasm_utils::storage::error::StorageError;

#[derive(Debug, Error)]
pub enum ClientStorageError {
    #[error("failed to use the storage: {source}")]
    StorageError {
        #[from]
        source: StorageError,
    },

    #[error("{typ} cryptographic key is not available in storage")]
    CryptoKeyNotInStorage { typ: String },

    #[error("the prior gateway details are not available in the storage")]
    GatewayDetailsNotInStorage,
}

impl From<ClientStorageError> for JsValue {
    fn from(value: ClientStorageError) -> Self {
        simple_js_error(value.to_string())
    }
}
