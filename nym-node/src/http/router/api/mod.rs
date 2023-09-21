// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};

pub mod v1;

pub(crate) mod routes {
    pub(crate) const V1: &str = "/v1";
    pub(crate) const SWAGGER: &str = "/swagger";
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub v1_config: v1::Config,
}

pub(super) fn routes(config: Config) -> Router {
    Router::new().nest_service(routes::V1, v1::routes(config.v1_config))
    // .nest(routes::SWAGGER, openapi::route())
    // .nest(routes::SWAGGER, openapi::route())
}

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    #[default]
    Json,
    Yaml,
}

impl Output {
    pub fn to_response<T: Serialize + 'static>(self, data: T) -> Box<dyn IntoResponse> {
        match self {
            Output::Json => Box::new(Json(data)),
            Output::Yaml => Box::new(Yaml(data)),
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
