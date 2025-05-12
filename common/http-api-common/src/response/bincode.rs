// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::response::{error_response, ResponseWrapper};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use bytes::{BufMut, BytesMut};
use serde::Serialize;

#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Bincode<T>(pub(crate) ResponseWrapper<T>);

impl<T> From<T> for Bincode<T> {
    fn from(response: T) -> Self {
        Bincode(ResponseWrapper::new(response).with_header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/bincode"),
        ))
    }
}

impl<T> IntoResponse for Bincode<T>
where
    T: Serialize,
{
    // replicates axum's Json
    fn into_response(self) -> Response {
        use bincode::Options;
        let mut buf = BytesMut::with_capacity(128).writer();

        match crate::make_bincode_serializer().serialize_into(&mut buf, &self.0.data) {
            Ok(()) => (self.0.headers, buf.into_inner().freeze()).into_response(),
            Err(err) => error_response(err),
        }
    }
}
