// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::GeoLocatorError;
use crate::ip_info_lookup::models::LocationResponse;
use http::{Method, StatusCode};
use reqwest::{Request, RequestBuilder};
use std::net::IpAddr;
use tracing::log::error;
use url::Url;
use zeroize::Zeroizing;

pub(crate) struct IpInfoClient {
    client: reqwest::Client,
    token: Zeroizing<String>,
}

fn ip_info_url() -> Url {
    // SAFETY: this hardcoded url is valid
    #[allow(clippy::unwrap_used)]
    "https://ipinfo.io".parse().unwrap()
}

impl IpInfoClient {
    pub(crate) async fn locate(&self, ip: IpAddr) -> Result<LocationResponse, GeoLocatorError> {
        let mut url = ip_info_url();
        url.path_segments_mut().unwrap().push(&ip.to_string());

        let request = Request::new(Method::GET, url);
        let request = RequestBuilder::from_parts(self.client.clone(), request)
            .query(&[("token", self.token.as_str())]);

        let response = request
            .send()
            .await
            .map_err(|source| GeoLocatorError::IpInfoRequestFailure { source })?;
        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(GeoLocatorError::IpInfoRateLimit);
        }

        let raw_response = response
            .text()
            .await
            .map_err(|source| GeoLocatorError::IpInfoRequestFailure { source })?;
        if !status.is_success() {
            error!("ipinfo request failed with status {status}: {raw_response}")
        }

        serde_json::from_str(&raw_response)
            .map_err(|source| GeoLocatorError::IpInfoResponseDeserialisationFailure { source })
    }
}
