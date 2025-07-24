// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AvailableBandwidth {
    pub(crate) value: i64,
}

pub(crate) type AxumResult<T> = Result<T, AxumErrorResponse>;

pub(crate) struct AxumErrorResponse {
    message: String,
    status: StatusCode,
}

impl AxumErrorResponse {
    pub(crate) fn bad_request(msg: impl Display) -> Self {
        Self {
            message: msg.to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

impl axum::response::IntoResponse for AxumErrorResponse {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
