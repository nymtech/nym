// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Unified error type for all v1 API endpoints.
/// The `Display` message from each variant is used as the HTTP response body.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("agent information not found")]
    AgentNotFound,

    #[error("failed to announce agent to the network monitors contract")]
    ContractFailure,

    #[error("failed to read or write data from the database")]
    StorageFailure,

    #[error("some of the stored data is malformed and could not be parsed")]
    MalformedStoredData,

    #[error("agent hasn't been announced to the contract - can't assign testruns")]
    AgentNotAnnounced,

    #[error("no test run found with the requested id")]
    TestRunNotFound,

    #[error("no nym-node found with the requested node id")]
    NymNodeNotFound,
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        use ApiError::*;

        match self {
            AgentNotFound | AgentNotAnnounced => StatusCode::BAD_REQUEST,
            TestRunNotFound | NymNodeNotFound => StatusCode::NOT_FOUND,
            ContractFailure | StorageFailure | MalformedStoredData => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code(), self.to_string()).into_response()
    }
}
