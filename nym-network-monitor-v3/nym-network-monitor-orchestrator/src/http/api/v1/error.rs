// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AgentAnnounceError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Unified error type for all v1 API endpoints.
/// The `Display` message from each variant is used as the HTTP response body.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("no available ports on this host")]
    NoPortsAvailable,

    #[error("agent information not found")]
    AgentNotFound,

    #[error("noise key does not match the one used during port assignment")]
    NoiseKeyMismatch,

    #[error("failed to announce agent to the network monitors contract")]
    ContractFailure,

    #[error("failed to read or write data from the database")]
    StorageFailure,

    #[error("some of the stored data is malformed and could not be parsed")]
    MalformedStoredData,

    #[error("agent hasn't been announced to the contract - can't assign testruns")]
    AgentNotAnnounced,

    #[error("this endpoint has not been implemented yet")]
    Unimplemented,
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        use ApiError::*;

        match self {
            NoPortsAvailable => StatusCode::SERVICE_UNAVAILABLE,
            AgentNotFound | NoiseKeyMismatch | AgentNotAnnounced => StatusCode::BAD_REQUEST,
            ContractFailure | StorageFailure | MalformedStoredData => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Unimplemented => StatusCode::NOT_IMPLEMENTED,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code(), self.to_string()).into_response()
    }
}

impl From<AgentAnnounceError> for ApiError {
    fn from(err: AgentAnnounceError) -> Self {
        match err {
            AgentAnnounceError::NotFound => ApiError::AgentNotFound,
            AgentAnnounceError::NoiseKeyMismatch => ApiError::NoiseKeyMismatch,
        }
    }
}
