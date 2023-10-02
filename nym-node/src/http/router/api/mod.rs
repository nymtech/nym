// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymNodeError;
use crate::http::api::v1::node::types::HostInformation;
use crate::http::state::AppState;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use bytes::{BufMut, BytesMut};
use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use utoipa::{IntoParams, ToSchema};

pub mod v1;

pub(crate) mod routes {
    pub(crate) const V1: &str = "/v1";
}

#[derive(Debug, Clone)]
pub struct Config {
    pub v1_config: v1::Config,
}

pub(super) fn routes(config: Config) -> Router<AppState> {
    Router::new().nest(routes::V1, v1::routes(config.v1_config))
}

#[derive(Debug, Clone, ToSchema, Serialize)]
#[aliases(SignedHostInformation = SignedData<HostInformation>)]
pub struct SignedData<T> {
    // #[serde(flatten)]
    pub data: T,
    pub signature: String,
}

impl<T> SignedData<T> {
    pub fn new(data: T, key: &identity::PrivateKey) -> Result<Self, NymNodeError>
    where
        T: Serialize,
    {
        let plaintext = serde_json::to_string(&data)?;
        let signature = key.sign(plaintext).to_base58_string();
        Ok(SignedData { data, signature })
    }

    pub fn verify(&self, key: &identity::PublicKey) -> bool
    where
        T: Serialize,
    {
        let Ok(plaintext) = serde_json::to_string(&self.data) else {
            return false;
        };
        let Ok(signature) = identity::Signature::from_base58_string(&self.signature) else {
            return false;
        };

        key.verify(plaintext, &signature).is_ok()
    }
}

impl<T> Deref for SignedData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
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
