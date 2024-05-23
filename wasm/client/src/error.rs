// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use wasm_client_core::error::WasmCoreError;
use wasm_client_core::topology::WasmTopologyError;
use wasm_client_core::ClientCoreError;
use wasm_utils::wasm_error;

#[cfg(feature = "node-tester")]
use nym_node_tester_utils::error::NetworkTestingError;

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

    #[cfg(feature = "node-tester")]
    #[error("failed to test the node: {source}")]
    NodeTestingFailure {
        #[from]
        source: NetworkTestingError,
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
