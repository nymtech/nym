// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
pub use nym_node_requests::api::ErrorResponse;
use utoipa::ToResponse;

#[derive(Debug, Clone, ToResponse)]
#[response(description = "Error response with additional message")]
pub(crate) struct RequestError {
    pub(crate) inner: ErrorResponse,

    pub(crate) status: StatusCode,
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

    pub(crate) fn new_status(status: StatusCode) -> Self {
        RequestError {
            inner: ErrorResponse {
                message: String::new(),
            },
            status,
        }
    }

    pub(crate) fn from_err<E: std::error::Error>(err: E, status: StatusCode) -> Self {
        Self::new(err.to_string(), status)
    }
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        (self.status, Json(self.inner)).into_response()
    }
}
