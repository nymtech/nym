// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_utils::simple_js_error;
use wasm_utils::storage::error::StorageError;

#[derive(Debug, Error)]
pub enum ExtensionStorageError {
    #[error("serialization failure: {source}")]
    JsonError {
        #[from]
        source: serde_wasm_bindgen::Error,
    },

    #[error("failed to use the storage: {source}")]
    StorageError {
        #[from]
        source: StorageError,
    },

    #[error("there's already a stored mnemonic with name {name}")]
    DuplicateMnemonic { name: String },

    #[error("the provided mnemonic is malformed: {source}")]
    InvalidMnemonic {
        #[from]
        source: bip39::Error,
    },
}

impl From<ExtensionStorageError> for JsValue {
    fn from(value: ExtensionStorageError) -> Self {
        simple_js_error(value.to_string())
    }
}
