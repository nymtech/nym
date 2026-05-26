// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_wasm_client_core::error::WasmCoreError;
use nym_wasm_client_core::topology::WasmTopologyError;
use nym_wasm_client_core::ClientCoreError;
use nym_wasm_utils::wasm_error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WasmClientError {
    #[error(transparent)]
    CoreError {
        #[from]
        source: WasmCoreError,
    },

    #[error("failed to parse mix config options: {source}")]
    MalformedConfigOptions {
        #[from]
        source: serde_wasm_bindgen::Error,
    },

    #[error("provided topology was malformed: {source}")]
    InvalidTopology {
        #[from]
        source: WasmTopologyError,
    },
}

// I dislike this so much - there must be a better way.
impl From<ClientCoreError> for WasmClientError {
    fn from(value: ClientCoreError) -> Self {
        WasmClientError::CoreError {
            source: WasmCoreError::BaseClientError { source: value },
        }
    }
}

wasm_error!(WasmClientError);
