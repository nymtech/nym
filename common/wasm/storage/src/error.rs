// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_wasm_utils::error::simple_js_error;
use serde_wasm_bindgen::Error;
use thiserror::Error;
use wasm_bindgen::JsValue;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("{0}")]
    Json(String),

    #[error("storage failure: {message}")]
    InternalStorageFailure { message: String },

    #[error("failed to open the db file: {message}")]
    DbOpenFailure { message: String },

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

impl From<indexed_db_futures::error::Error> for StorageError {
    fn from(value: indexed_db_futures::error::Error) -> Self {
        StorageError::InternalStorageFailure {
            message: value.to_string(),
        }
    }
}

impl From<indexed_db_futures::error::OpenDbError> for StorageError {
    fn from(value: indexed_db_futures::error::OpenDbError) -> Self {
        StorageError::DbOpenFailure {
            message: value.to_string(),
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
