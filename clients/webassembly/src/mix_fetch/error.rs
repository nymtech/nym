// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::request_correlator::RequestId;
use js_sys::Promise;
use nym_http_requests::error::MixHttpRequestError;
use nym_ordered_buffer::OrderedMessageError;
use nym_socks5_requests::ConnectionError;
use std::time::Duration;
use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_utils::simple_js_error;

#[derive(Debug, Error)]
pub enum MixFetchError {
    #[error("the provided request was invalid: {source}")]
    InvalidRequest {
        #[from]
        source: MixHttpRequestError,
    },

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

    #[error("request {id} timed out after {timeout:?}")]
    Timeout { id: RequestId, timeout: Duration },
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
