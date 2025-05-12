// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::response::{error_response, ResponseWrapper};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use bytes::{BufMut, BytesMut};
use serde::Serialize;

#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Yaml<T>(pub(crate) ResponseWrapper<T>);

impl<T> From<T> for Yaml<T> {
    fn from(response: T) -> Self {
        Yaml(ResponseWrapper::new(response).with_header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/yaml"),
        ))
    }
}

impl<T> IntoResponse for Yaml<T>
where
    T: Serialize,
{
    // replicates axum's Json
    fn into_response(self) -> Response {
        let mut buf = BytesMut::with_capacity(128).writer();
        match serde_yaml::to_writer(&mut buf, &self.0.data) {
            Ok(()) => (self.0.headers, buf.into_inner().freeze()).into_response(),
            Err(err) => error_response(err),
        }
    }
}
