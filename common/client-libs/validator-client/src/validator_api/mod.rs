// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
mod routes;

use crate::validator_api::error::ValidatorAPIClientError;

use serde::Deserialize;
use url::Url;

pub const VALIDATOR_API_PORT: u16 = 8080;
pub const VALIDATOR_API_CACHE_VERSION: &str = "/v1";
pub(crate) const VALIDATOR_API_MIXNODES: &str = "/mixnodes";
pub(crate) const VALIDATOR_API_GATEWAYS: &str = "/gateways";

pub struct Client {
    reqwest_client: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        Client::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let reqwest_client = reqwest::Client::new();
        Self { reqwest_client }
    }

    pub async fn query_validator_api<T>(
        &self,
        query: String,
        validator_url: &Url,
    ) -> Result<T, ValidatorAPIClientError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let mut validator_api_url = validator_url.clone();
        validator_api_url
            .set_port(Some(VALIDATOR_API_PORT))
            .unwrap();
        let query_url = format!("{}{}", validator_api_url.as_str(), query);
        Ok(self
            .reqwest_client
            .get(query_url)
            .send()
            .await?
            .json()
            .await?)
    }
}
