// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::CredentialProxyError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use nym_credential_proxy_requests::api::v1::ErrorResponse;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone)]
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

    pub fn new_plain_error(err: CredentialProxyError) -> Self {
        Self::from_err(err, StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn new_server_error(err: CredentialProxyError, uuid: Uuid) -> Self {
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

pub fn db_failure<T>(err: CredentialProxyError, uuid: Uuid) -> Result<T, RequestError> {
    warn!("db failure: {err}");
    Err(RequestError::new_with_uuid(
        format!("oh no, something went wrong {err}"),
        uuid,
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}
