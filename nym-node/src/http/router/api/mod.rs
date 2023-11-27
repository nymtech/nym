// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub mod v1;

use crate::http::api::v1::gateway::client_interfaces::wireguard::WireguardAppState;
use crate::http::state::AppState;
pub use nym_node_requests::api as api_requests;
use nym_node_requests::routes;

#[derive(Debug, Clone)]
pub struct Config {
    pub v1_config: v1::Config,
}

pub(super) fn routes(config: Config, initial_wg_state: WireguardAppState) -> Router<AppState> {
    Router::new().nest(
        routes::api::V1,
        v1::routes(config.v1_config, initial_wg_state),
    )
}

#[derive(Debug, Clone, ToSchema)]
pub enum FormattedResponse<T> {
    Json(Json<T>),
    Yaml(Yaml<T>),
}

impl<T> IntoResponse for FormattedResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            FormattedResponse::Json(json_response) => json_response.into_response(),
            FormattedResponse::Yaml(yaml_response) => yaml_response.into_response(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    #[default]
    Json,
    Yaml,
}

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone, IntoParams, ToSchema)]
#[serde(default)]
pub struct OutputParams {
    pub output: Option<Output>,
}

impl Output {
    pub fn to_response<T: Serialize>(self, data: T) -> FormattedResponse<T> {
        match self {
            Output::Json => FormattedResponse::Json(Json(data)),
            Output::Yaml => FormattedResponse::Yaml(Yaml(data)),
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
