// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::http::header::IntoHeaderName;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

#[cfg(feature = "bincode")]
pub mod bincode;
pub mod json;
pub mod yaml;

pub use json::Json;
pub use yaml::Yaml;

#[cfg(feature = "bincode")]
pub use bincode::Bincode;

#[derive(Debug, Clone, Default)]
pub(crate) struct ResponseWrapper<T> {
    data: T,
    headers: HeaderMap,
}

impl<T> ResponseWrapper<T> {
    pub(crate) fn new(response: T) -> ResponseWrapper<T> {
        ResponseWrapper {
            data: response,
            headers: Default::default(),
        }
    }

    pub(crate) fn with_header(
        mut self,
        name: impl IntoHeaderName,
        value: impl Into<HeaderValue>,
    ) -> Self {
        self.headers.insert(name, value.into());
        self
    }
}

#[derive(Debug, Clone)]
pub enum FormattedResponse<T> {
    Json(Json<T>),
    Yaml(Yaml<T>),
    #[cfg(feature = "bincode")]
    Bincode(Bincode<T>),
}

impl<T> FormattedResponse<T> {
    pub fn into_inner(self) -> T {
        match self {
            FormattedResponse::Json(inner) => inner.0.data,
            FormattedResponse::Yaml(inner) => inner.0.data,
            #[cfg(feature = "bincode")]
            FormattedResponse::Bincode(inner) => inner.0.data,
        }
    }

    pub fn with_header(self) -> FormattedResponse<T> {
        todo!()
    }
}

impl<T> IntoResponse for FormattedResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            FormattedResponse::Json(json_response) => json_response.into_response(),
            FormattedResponse::Yaml(yaml_response) => yaml_response.into_response(),
            #[cfg(feature = "bincode")]
            FormattedResponse::Bincode(bincode_response) => bincode_response.into_response(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum Output {
    #[default]
    Json,
    Yaml,
    #[cfg(feature = "bincode")]
    Bincode,
}

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams, utoipa::ToSchema))]
#[serde(default)]
pub struct OutputParams {
    pub output: Option<Output>,
}

impl Output {
    pub fn to_response<T: Serialize>(self, data: T) -> FormattedResponse<T> {
        match self {
            Output::Json => FormattedResponse::Json(Json::from(data)),
            Output::Yaml => FormattedResponse::Yaml(Yaml::from(data)),
            #[cfg(feature = "bincode")]
            Output::Bincode => FormattedResponse::Bincode(Bincode::from(data)),
        }
    }
}

pub(crate) fn error_response<E: ToString>(err: E) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
        )],
        err.to_string(),
    )
        .into_response()
}
