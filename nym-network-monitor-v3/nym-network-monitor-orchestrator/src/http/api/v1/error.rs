// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AgentAnnounceError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Unified error type for all v1 API endpoints.
/// The `Display` message from each variant is used as the HTTP response body.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    // -- port request --
    #[error("no available ports on this host")]
    NoPortsAvailable,

    // -- agent announce --
    #[error("agent information not found")]
    AgentNotFound,

    #[error("noise key does not match the one used during port assignment")]
    NoiseKeyMismatch,

    #[error("failed to announce agent to the network monitors contract")]
    ContractFailure,
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NoPortsAvailable => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::AgentNotFound | ApiError::NoiseKeyMismatch => StatusCode::BAD_REQUEST,
            ApiError::ContractFailure => StatusCode::INTERNAL_SERVER_ERROR,
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
