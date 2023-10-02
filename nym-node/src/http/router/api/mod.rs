// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
    // .nest(routes::SWAGGER, openapi::route())
    // .nest(routes::SWAGGER, openapi::route())
}

// #[derive(Debug, Clone, ToSchema)]
// pub struct SignedResponse<T>{
//     encoded_signature: String,
//     data: T,
// }

#[derive(Debug, Clone, ToSchema)]
pub struct SignedResponse<T> {
    pub response: T,
    pub signature: String,
}

impl<T> SignedResponse<T> {
    pub fn new(response: T, key: &identity::PrivateKey) -> serde_json::Result<Self>
    where
        T: Serialize,
    {
        let plaintext = serde_json::to_string(&response)?;
        let signature = key.sign(plaintext).to_base58_string();
        Ok(SignedResponse {
            response,
            signature,
        })
    }

    pub fn verify(&self, key: &identity::PublicKey) -> bool
    where
        T: Serialize,
    {
        let Ok(plaintext) = serde_json::to_string(&self.response) else {
            return false;
        };
        let Ok(signature) = identity::Signature::from_base58_string(&self.signature) else {
            return false;
        };

        key.verify(plaintext, &signature).is_ok()
    }
}

impl<T> Deref for SignedResponse<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.response
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
