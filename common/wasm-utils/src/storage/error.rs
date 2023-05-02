// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::simple_js_error;
use indexed_db_futures::web_sys::DomException;
use thiserror::Error;
use wasm_bindgen::JsValue;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Json(#[from] serde_wasm_bindgen::Error),

    #[error("DomException {name} ({code}): {message}")]
    DomException {
        /// DomException code
        code: u16,
        /// Specific name of the DomException
        name: String,
        /// Message given to the DomException
        message: String,
    },

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
