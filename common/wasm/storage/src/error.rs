// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use indexed_db_futures::web_sys::DomException;
use serde_wasm_bindgen::Error;
use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_utils::error::simple_js_error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("{0}")]
    Json(String),

    #[error("DomException {name} ({code}): {message}")]
    DomException {
        /// DomException code
        code: u16,
        /// Specific name of the DomException
        name: String,
        /// Message given to the DomException
        message: String,
    },

    #[error("FATAL ERROR: storage key is somehow present {count} times in the table!")]
    DuplicateKey { count: u32 },

    #[error("encountered issue with our storage encryption layer: {source}")]
    CryptoStorageError {
        #[from]
        source: nym_store_cipher::Error,
    },

    #[error(
        "attempted to unlock an existing encrypted client store without providing a passphrase"
    )]
    NoPassphraseProvided,

    #[error("attempted to access an existing unencrypted client store with a passphrase")]
    UnexpectedPassphraseProvided,
}

impl From<StorageError> for JsValue {
    fn from(value: StorageError) -> Self {
        simple_js_error(value.to_string())
    }
}

impl From<DomException> for StorageError {
    fn from(value: DomException) -> StorageError {
        StorageError::DomException {
            name: value.name(),
            message: value.message(),
            code: value.code(),
        }
    }
}

// covert it to String so that we wouldn't store `JsValue` indirectly
// thus not making us !Send + !Sync
// (we could have done bunch of target locking elsewhere instead, but this solution is way simpler)
impl From<serde_wasm_bindgen::Error> for StorageError {
    fn from(value: Error) -> Self {
        StorageError::Json(value.to_string())
    }
}
