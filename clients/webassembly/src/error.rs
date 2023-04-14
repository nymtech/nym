// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::topology::WasmTopologyError;
use nym_crypto::asymmetric::identity::Ed25519RecoveryError;
use nym_gateway_client::error::GatewayClientError;
use thiserror::Error;
use wasm_bindgen::JsValue;
use node_tester_utils::error::NetworkTestingError;
use wasm_utils::simple_js_error;

// might as well start using well-defined error enum...
#[derive(Debug, Error)]
pub enum WasmClientError {
    #[error("The provided gateway identity is invalid: {source}")]
    InvalidGatewayIdentity { source: Ed25519RecoveryError },

    #[error("Gateway communication failure: {source}")]
    GatewayClientError {
        #[from]
        source: GatewayClientError,
    },

    #[error("The provided topology was invalid: {source}")]
    WasmTopologyError {
        #[from]
        source: WasmTopologyError,
    },
    
    #[error("failed to test the node: {source}")]
    NodeTestingFailure {
        #[from]
        source: NetworkTestingError
    }
}

impl From<WasmClientError> for JsValue {
    fn from(value: WasmClientError) -> Self {
        simple_js_error(value.to_string())
    }
}
