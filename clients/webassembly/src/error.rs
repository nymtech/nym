// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::topology::WasmTopologyError;
use js_sys::Promise;
use node_tester_utils::error::NetworkTestingError;
use nym_crypto::asymmetric::identity::Ed25519RecoveryError;
use nym_gateway_client::error::GatewayClientError;
use nym_validator_client::ValidatorClientError;
use thiserror::Error;
use wasm_bindgen::JsValue;
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

    #[error("failed to query nym api: {source}")]
    NymApiError {
        #[from]
        source: ValidatorClientError,
    },

    #[error("The provided topology was invalid: {source}")]
    WasmTopologyError {
        #[from]
        source: WasmTopologyError,
    },

    #[error("failed to test the node: {source}")]
    NodeTestingFailure {
        #[from]
        source: NetworkTestingError,
    },

    #[error("{raw} is not a valid url: {source}")]
    MalformedUrl {
        raw: String,
        source: url::ParseError,
    },
}

impl WasmClientError {
    pub fn into_rejected_promise(self) -> Promise {
        self.into()
    }
}

impl From<WasmClientError> for JsValue {
    fn from(value: WasmClientError) -> Self {
        simple_js_error(value.to_string())
    }
}

impl From<WasmClientError> for Promise {
    fn from(value: WasmClientError) -> Self {
        Promise::reject(&value.into())
    }
}
