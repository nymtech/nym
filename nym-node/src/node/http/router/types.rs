// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
pub use nym_node_requests::api::ErrorResponse;

#[derive(Debug, Clone)]
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
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        (self.status, Json(self.inner)).into_response()
    }
}
