// Copyright 2023-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bincode::Options;
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};

pub mod middleware;

#[derive(Debug, Clone)]
pub enum FormattedResponse<T> {
    Json(Json<T>),
    Yaml(Yaml<T>),
    #[cfg(feature = "bincode")]
    Bincode(Bincode<T>),
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
            Output::Json => FormattedResponse::Json(Json(data)),
            Output::Yaml => FormattedResponse::Yaml(Yaml(data)),
            #[cfg(feature = "bincode")]
            Output::Bincode => FormattedResponse::Bincode(Bincode(data)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Yaml<T>(pub T);

impl<T> From<T> for Yaml<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> IntoResponse for Yaml<T>
where
    T: Serialize,
{
    // replicates axum's Json
    fn into_response(self) -> Response {
        let mut buf = BytesMut::with_capacity(128).writer();
        match serde_yaml::to_writer(&mut buf, &self.0) {
            Ok(()) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/yaml"),
                )],
                buf.into_inner().freeze(),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}

// be explicit about those values because bincode uses different defaults in different places
#[cfg(feature = "bincode")]
pub fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_little_endian()
        .with_varint_encoding()
}

#[derive(Debug, Clone, Copy, Default)]
#[must_use]
#[cfg(feature = "bincode")]
pub struct Bincode<T>(pub T);

#[cfg(feature = "bincode")]
impl<T> IntoResponse for Bincode<T>
where
    T: Serialize,
{
    // replicates axum's Json
    fn into_response(self) -> Response {
        let mut buf = BytesMut::with_capacity(128).writer();

        match make_bincode_serializer().serialize_into(&mut buf, &self.0) {
            Ok(()) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/bincode"),
                )],
                buf.into_inner().freeze(),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}
