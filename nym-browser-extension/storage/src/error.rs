// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use wasm_storage::error::StorageError;
use wasm_utils::wasm_error;

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

wasm_error!(ExtensionStorageError);
