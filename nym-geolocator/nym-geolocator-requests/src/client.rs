// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{MeasurementResponse, SignedCheckRequest};
use crate::routes::api::v1::geolocation::request_check_absolute;
use nym_http_api_client::{ApiClient, HttpClientError};

/// A client for the endpoints a node itself calls.
///
/// Lives here rather than in each caller so the route constants and the request types stay
/// together: a caller assembling the path from string literals of its own would keep working while
/// silently addressing a route that had since moved.
#[derive(Clone, Debug)]
pub struct GeolocatorClient {
    inner: nym_http_api_client::Client,
}

impl GeolocatorClient {
    pub fn new(base_url: &str) -> Result<GeolocatorClient, HttpClientError> {
        Ok(GeolocatorClient {
            inner: nym_http_api_client::Client::builder(base_url)?.build()?,
        })
    }

    /// Ask the agent to measure the requesting node now.
    pub async fn request_check(
        &self,
        request: &SignedCheckRequest,
    ) -> Result<MeasurementResponse, HttpClientError> {
        self.inner
            .post_json_data_to(request_check_absolute(), request)
            .await
    }
}
