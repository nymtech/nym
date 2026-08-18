// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
pub(crate) use nym_geolocator_requests::models::ErrorResponse;

#[derive(Debug, Clone)]
pub(crate) struct RequestError {
    inner: ErrorResponse,

    status: StatusCode,
}

impl RequestError {
    pub(crate) fn new<S: Into<String>>(message: S, status: StatusCode) -> Self {
        RequestError {
            inner: ErrorResponse {
                message: message.into(),
            },
            status,
        }
    }

    pub(crate) fn not_found<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::NOT_FOUND)
    }

    pub(crate) fn unauthorised<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::UNAUTHORIZED)
    }

    pub(crate) fn too_many_requests<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::TOO_MANY_REQUESTS)
    }

    pub(crate) fn bad_request<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::BAD_REQUEST)
    }

    pub(crate) fn forbidden<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::FORBIDDEN)
    }

    /// Somebody else got there first. Distinct from a failure, since the caller's declaration is
    /// on chain either way and a node relaying to several agents will see this from all but one.
    pub(crate) fn conflict<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::CONFLICT)
    }

    /// Something upstream of this service failed: the node itself, the lookup provider or the
    /// chain. Deliberately coarse towards the caller, with the detail going to the logs, since a
    /// node-signed caller has no business learning which of our dependencies is unhealthy.
    pub(crate) fn upstream_failure<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::BAD_GATEWAY)
    }

    /// This service was busy with other work and did nothing for the caller. Distinct from a
    /// failure, since nothing was attempted and an immediate retry is free.
    pub(crate) fn busy<S: Into<String>>(message: S) -> Self {
        RequestError::new(message, StatusCode::SERVICE_UNAVAILABLE)
    }
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        (self.status, Json(self.inner)).into_response()
    }
}
