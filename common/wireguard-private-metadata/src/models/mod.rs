// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use axum::http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) mod error;
pub(crate) mod version_1;

pub(crate) use version_1 as latest;

pub type Version = usize;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AvailableBandwidthResponse {
    pub(crate) version: Version,
    pub(crate) inner: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TopUpRequest {
    pub(crate) version: Version,
    pub(crate) inner: Vec<u8>,
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
