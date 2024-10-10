// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use nym_vpn_api_requests::api::v1::ErrorResponse;
use utoipa::ToResponse;
use uuid::Uuid;

#[derive(Debug, Clone, ToResponse)]
#[response(description = "Error response with additional message")]
pub struct RequestError {
    pub inner: ErrorResponse,

    pub status: StatusCode,
}

impl RequestError {
    pub fn new<S: Into<String>>(message: S, status: StatusCode) -> Self {
        RequestError {
            inner: ErrorResponse {
                uuid: None,
                message: message.into(),
            },
            status,
        }
    }

    pub fn new_status(status: StatusCode) -> Self {
        RequestError {
            inner: ErrorResponse {
                uuid: None,
                message: String::new(),
            },
            status,
        }
    }

    pub fn new_server_error(err: VpnApiError, uuid: Uuid) -> Self {
        RequestError::new_with_uuid(err.to_string(), uuid, StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn new_with_uuid<S: Into<String>>(message: S, uuid: Uuid, status: StatusCode) -> Self {
        RequestError {
            inner: ErrorResponse {
                uuid: Some(uuid),
                message: message.into(),
            },
            status,
        }
    }

    pub fn from_err<E: std::error::Error>(err: E, status: StatusCode) -> Self {
        Self::new(err.to_string(), status)
    }
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        (self.status, Json(self.inner)).into_response()
    }
}
