// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::GeoLocatorError;
use crate::ip_info_lookup::models::LocationResponse;
use http::{Method, StatusCode};
use reqwest::{Request, RequestBuilder};
use std::net::IpAddr;
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
    pub(crate) fn new(token: String) -> Self {
        IpInfoClient {
            client: reqwest::Client::new(),
            token: Zeroizing::new(token),
        }
    }

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

        // a rejected request must fail rather than fall through to deserialisation. Every field
        // of `LocationResponse` has a default and unknown fields are ignored, so ipinfo's error
        // body parses perfectly happily into a location of country "" at 0,0. An expired token
        // would then not fail at all: it would quietly relocate the entire network to the Gulf
        // of Guinea and commit that to the chain as freshly checked fact
        if !status.is_success() {
            return Err(GeoLocatorError::IpInfoRequestRejected {
                status,
                body: raw_response,
            });
        }

        let response: LocationResponse = serde_json::from_str(&raw_response)
            .map_err(|source| GeoLocatorError::IpInfoResponseDeserialisationFailure { source })?;

        // ipinfo also answers 200 with those same empty defaults for an address it cannot place,
        // a bogon being the common case. That is an absent location, not a location, and the
        // difference is invisible once it has been written: an entry asserting country "" is
        // indistinguishable from a real answer to anything reading the contract
        if response.two_letter_iso_country_code.is_empty() {
            return Err(GeoLocatorError::IpInfoNoLocationData);
        }

        Ok(response)
    }
}
