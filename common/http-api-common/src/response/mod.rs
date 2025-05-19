// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::http::header::IntoHeaderName;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::format_description::BorrowedFormatItem;
use time::macros::{format_description, offset};
use time::OffsetDateTime;

pub mod bincode;
pub mod json;
pub mod yaml;

pub use bincode::Bincode;
pub use json::Json;
pub use yaml::Yaml;

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

    pub(crate) fn map<U, F: FnOnce(T) -> U>(self, op: F) -> ResponseWrapper<U> {
        ResponseWrapper {
            data: op(self.data),
            headers: self.headers,
        }
    }

    #[must_use]
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
    Bincode(Bincode<T>),
}

impl<T> FormattedResponse<T> {
    pub fn into_inner(self) -> T {
        match self {
            FormattedResponse::Json(inner) => inner.0.data,
            FormattedResponse::Yaml(inner) => inner.0.data,
            FormattedResponse::Bincode(inner) => inner.0.data,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, op: F) -> FormattedResponse<U> {
        match self {
            FormattedResponse::Json(inner) => FormattedResponse::Json(inner.map(op)),
            FormattedResponse::Yaml(inner) => FormattedResponse::Yaml(inner.map(op)),
            FormattedResponse::Bincode(inner) => FormattedResponse::Bincode(inner.map(op)),
        }
    }

    #[must_use]
    pub fn with_header(
        self,
        name: impl IntoHeaderName,
        value: impl Into<HeaderValue>,
    ) -> FormattedResponse<T> {
        match self {
            FormattedResponse::Json(inner) => {
                FormattedResponse::Json(inner.with_header(name, value))
            }
            FormattedResponse::Yaml(inner) => {
                FormattedResponse::Yaml(inner.with_header(name, value))
            }
            FormattedResponse::Bincode(inner) => {
                FormattedResponse::Bincode(inner.with_header(name, value))
            }
        }
    }

    /// Set the `expires` header on the response to the provided expiration.
    /// Internally it will perform conversions to make sure the value is set in GMT offset,
    /// e.g. `Expires: Wed, 21 Oct 2015 07:28:00 GMT`
    #[must_use]
    pub fn with_expires_header(self, expiration: OffsetDateTime) -> FormattedResponse<T> {
        // as per RFC-7234 (section 5.3) EXPIRES header has to use value formatted
        // as defined in RFC-7231 (section 7.1.1.1)
        // (preferred format (IMF-fixdate) uses RFC-5322 (section 3.3)
        let formatted = format_rfc5352(expiration);

        // SAFETY: our formatted datetime doesn't contain forbidden characters
        #[allow(clippy::unwrap_used)]
        self.with_header(header::EXPIRES, HeaderValue::try_from(formatted).unwrap())
    }

    /// Work similarly to `with_expires_header`, but rather than setting explicit expiration value,
    /// it adds the provided time delta to the current time instead.
    #[must_use]
    pub fn with_expires_header_delta(self, expires_in: Duration) -> FormattedResponse<T> {
        self.with_expires_header(OffsetDateTime::now_utc() + expires_in)
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

// SAFETY: this hardcoded datetime formatter is valid
#[allow(clippy::unwrap_used)]
fn format_rfc5352(datetime: OffsetDateTime) -> String {
    // the time must be using GMT (UTC) offset
    let normalised = datetime.to_offset(offset!(UTC));
    normalised.format(&rfc5322()).unwrap()
}

// NOTE: this function is purposely not made public as it cannot guarantee caller
// has correctly ensured their date is using correct GMT offset
fn rfc5322() -> &'static [BorrowedFormatItem<'static>] {
    // D, d M Y H:i:s T
    format_description!(
        "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"
    )
}

#[cfg(test)]
mod tests {
    use crate::response::format_rfc5352;
    use time::macros::datetime;

    #[test]
    fn rfc5322_formatting() {
        let utc_date = datetime!(2021-08-23 12:13:14 UTC);
        let non_utc_date = datetime!(2021-08-23 12:13:14 -1);

        assert_eq!("Mon, 23 Aug 2021 12:13:14 GMT", format_rfc5352(utc_date));
        assert_eq!(
            "Mon, 23 Aug 2021 13:13:14 GMT",
            format_rfc5352(non_utc_date)
        );
    }
}
