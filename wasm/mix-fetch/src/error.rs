// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::harbourmaster::HarbourMasterApiError;
use crate::RequestId;
use nym_ordered_buffer::OrderedMessageError;
use nym_socks5_requests::ConnectionError;
use thiserror::Error;
use wasm_client_core::error::WasmCoreError;
use wasm_client_core::ClientCoreError;
use wasm_utils::wasm_error;

#[derive(Debug, Error)]
pub enum MixFetchError {
    #[error(transparent)]
    CoreError {
        #[from]
        source: WasmCoreError,
    },

    #[error("no public network requesters are not available on this network")]
    NoNetworkRequesters,

    #[error("could not query for the service providers: {source}")]
    HarbourmasterError {
        #[from]
        source: HarbourMasterApiError,
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

    #[error("mix fetch client has already been disconnected")]
    Disconnected,

    #[error("provided mix fetch url wasn't a string")]
    NotStringMixFetchUrl,

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
}

// I dislike this so much - there must be a better way.
impl From<ClientCoreError> for MixFetchError {
    fn from(value: ClientCoreError) -> Self {
        MixFetchError::CoreError {
            source: WasmCoreError::BaseClientError { source: value },
        }
    }
}

impl From<ConnectionError> for MixFetchError {
    fn from(value: ConnectionError) -> Self {
        MixFetchError::ConnectionError {
            network_requester_message: value.network_requester_error,
        }
    }
}

wasm_error!(MixFetchError);
