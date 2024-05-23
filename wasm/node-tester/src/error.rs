// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_node_tester_utils::error::NetworkTestingError;
use thiserror::Error;
use wasm_client_core::error::WasmCoreError;
use wasm_client_core::topology::WasmTopologyError;
use wasm_client_core::{ClientCoreError, GatewayClientError};
use wasm_utils::wasm_error;

#[derive(Debug, Error)]
pub enum NodeTesterError {
    #[error(
        "A node test is already in progress. Wait for it to finish before starting another one."
    )]
    TestInProgress,

    #[error(
        "both nymApi address and explicit topology were specified - please use only one of them"
    )]
    DuplicateTopologySource,

    #[error("neither nymApi address or explicit topology were specified")]
    NoTopologySource,

    #[error("could not parse provided tester arguments: {err}")]
    MalformedNodeTesterArguments { err: String },

    #[error(transparent)]
    CoreError {
        #[from]
        source: WasmCoreError,
    },

    #[error("provided topology was malformed: {source}")]
    InvalidTopology {
        #[from]
        source: WasmTopologyError,
    },

    #[error("failed to test the node: {source}")]
    NodeTestingFailure {
        #[from]
        source: NetworkTestingError,
    },

    #[error("experienced an error with the gateway connection: {source}")]
    GatewayConnectionError {
        #[from]
        source: GatewayClientError,
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

wasm_error!(NodeTesterError);
