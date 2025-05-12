// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::response::{error_response, ResponseWrapper};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use bytes::{BufMut, BytesMut};
use serde::Serialize;
use utoipa::gen::serde_json;

// don't use axum's Json directly as we need to be able to define custom headers
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Json<T>(pub(crate) ResponseWrapper<T>);

impl<T> From<T> for Json<T> {
    fn from(response: T) -> Self {
        Json(ResponseWrapper::new(response).with_header(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        ))
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut buf = BytesMut::with_capacity(128).writer();

        match serde_json::to_writer(&mut buf, &self.0.data) {
            Ok(()) => (self.0.headers, buf.into_inner().freeze()).into_response(),
            Err(err) => error_response(err),
        }
    }
}
