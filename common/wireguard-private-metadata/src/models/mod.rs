// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Display, Formatter};

use axum::http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub(crate) mod error;
pub(crate) mod interface;
#[cfg(test)]
pub(crate) mod v0; // dummy version, only for filling boilerplate code for update/downgrade and testing
pub mod v1;

pub(crate) use v1 as latest;

use crate::models::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub enum Version {
    #[cfg(test)]
    /// only used for testing purposes, don't include it in your matching arms
    V0,
    V1,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Request {
    pub(crate) version: Version,
    pub(crate) inner: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Response {
    pub(crate) version: Version,
    pub(crate) inner: Vec<u8>,
}

pub trait Extract<T> {
    fn extract(&self) -> Result<(T, Version), Error>;
}

pub trait Construct<T>: Sized {
    fn construct(info: T, version: Version) -> Result<Self, Error>;
}

pub(crate) type AxumResult<T> = Result<T, AxumErrorResponse>;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ErrorResponse {
    pub message: String,
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

pub(crate) struct AxumErrorResponse {
    message: ErrorResponse,
    status: StatusCode,
}

impl AxumErrorResponse {
    pub(crate) fn bad_request(msg: impl Display) -> Self {
        Self {
            message: ErrorResponse {
                message: msg.to_string(),
            },
            status: StatusCode::BAD_REQUEST,
        }
    }
}

impl axum::response::IntoResponse for AxumErrorResponse {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message.message.to_string()).into_response()
    }
}
