// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use js_sys::Promise;
use nym_node_tester_utils::error::NetworkTestingError;
use thiserror::Error;
use wasm_client_core::error::WasmCoreError;
use wasm_client_core::{ClientCoreError, GatewayClientError};
use wasm_utils::wasm_error;

#[derive(Debug, Error)]
pub enum NodeTesterError {
    #[error(
        "A node test is already in progress. Wait for it to finish before starting another one."
    )]
    TestInProgress,

    #[error(transparent)]
    CoreError {
        #[from]
        source: WasmCoreError,
    },

    #[error("failed to test the node: {source}")]
    NodeTestingFailure {
        #[from]
        source: NetworkTestingError,
    },
}

// I dislike this so much - there must be a better way.
impl From<ClientCoreError> for NodeTesterError {
    fn from(value: ClientCoreError) -> Self {
        NodeTesterError::CoreError {
            source: WasmCoreError::BaseClientError { source: value },
        }
    }
}

impl From<GatewayClientError> for NodeTesterError {
    fn from(value: GatewayClientError) -> Self {
        ClientCoreError::GatewayClientError(value).into()
    }
}

wasm_error!(NodeTesterError);
