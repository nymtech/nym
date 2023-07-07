// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use crate::mix_fetch::RequestId;
use crate::storage::error::ClientStorageError;
use js_sys::Promise;
use nym_client_core::error::ClientCoreError;
use nym_ordered_buffer::OrderedMessageError;
use nym_socks5_requests::ConnectionError;
use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_utils::simple_js_error;

#[derive(Debug, Error)]
pub enum MixFetchError {
    // TODO: this shouldn't be here. whatever is shared between the two should be moved to separate enum
    #[error(transparent)]
    WasmClientError(#[from] WasmClientError),

    #[error("experienced an issue with internal client components: {source}")]
    BaseClientError {
        #[from]
        source: ClientCoreError,
    },

    #[error("failed to parse mix fetch config options: {source}")]
    MalformedConfigOptions {
        #[from]
        source: serde_wasm_bindgen::Error,
    },

    #[error("mix fetch hasn't been initialised")]
    Uninitialised,

    #[error("mix fetch has already been initialised before")]
    AlreadyInitialised,

    #[error("provided mix fetch url wasn't a string")]
    NotStringMixFetchUrl,

    #[error("the opaque URL origin is unsupported")]
    UnsupportedOrigin,

    #[error("request {request_id} has been aborted")]
    AbortedRequest { request_id: RequestId },

    #[error("provided mix fetch url was malformed: {0}")]
    MalformedMixFetchUrl(#[from] url::ParseError),

    // the maximum value is u32::MAX which equals to over 49days, which is MORE than enough
    #[error("attempted to set request timeout to {timeout_ms}ms")]
    InvalidTimeoutValue { timeout_ms: u128 },

    #[error("network requester has rejected our request: {network_requester_message}")]
    ConnectionError { network_requester_message: String },

    #[error("failed to reconstruct the response: {source}")]
    MalformedData {
        #[from]
        source: OrderedMessageError,
    },

    #[error("received multiple messages about the remote socket being closed for request {request}. The first was on seq {first} and the other on {other}")]
    DuplicateSocketClosure {
        request: RequestId,
        first: u64,
        other: u64,
    },

    #[error(transparent)]
    StorageError {
        #[from]
        source: ClientStorageError,
    },
}

impl MixFetchError {
    pub fn into_rejected_promise(self) -> Promise {
        self.into()
    }
}

impl From<MixFetchError> for JsValue {
    fn from(value: MixFetchError) -> Self {
        simple_js_error(value.to_string())
    }
}

impl From<MixFetchError> for Promise {
    fn from(value: MixFetchError) -> Self {
        Promise::reject(&value.into())
    }
}

impl From<ConnectionError> for MixFetchError {
    fn from(value: ConnectionError) -> Self {
        MixFetchError::ConnectionError {
            network_requester_message: value.network_requester_error,
        }
    }
}
