// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::validator_api::error::ValidatorAPIError;
use coconut_interface::{BlindSignRequestBody, BlindedSignatureResponse, VerificationKeyResponse};
use mixnet_contract::{GatewayBond, MixNodeBond};
use serde::{Deserialize, Serialize};
use url::Url;

pub mod error;
pub(crate) mod routes;

type PathSegments<'a> = &'a [&'a str];

pub struct Client {
    url: Url,
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new(url: Url) -> Self {
        let reqwest_client = reqwest::Client::new();
        Self {
            url,
            reqwest_client,
        }
    }

    pub fn change_url(&mut self, new_url: Url) {
        self.url = new_url
    }

    async fn query_validator_api<T>(&self, path: PathSegments<'_>) -> Result<T, ValidatorAPIError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let url = create_api_url(&self.url, path);
        Ok(self.reqwest_client.get(url).send().await?.json().await?)
    }

    async fn post_validator_api<B, T>(
        &self,
        path: PathSegments<'_>,
        json_body: &B,
    ) -> Result<T, ValidatorAPIError>
    where
        B: Serialize + ?Sized,
        for<'a> T: Deserialize<'a>,
    {
        let url = create_api_url(&self.url, path);
        Ok(self
            .reqwest_client
            .post(url)
            .json(json_body)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorAPIError> {
        self.query_validator_api(&[routes::API_VERSION, routes::MIXNODES])
            .await
    }

    pub async fn get_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorAPIError> {
        self.query_validator_api(&[routes::API_VERSION, routes::GATEWAYS])
            .await
    }

    pub async fn get_active_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorAPIError> {
        self.query_validator_api(&[routes::API_VERSION, routes::MIXNODES, routes::ACTIVE])
            .await
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorAPIError> {
        self.post_validator_api(
            &[routes::API_VERSION, routes::COCONUT_BLIND_SIGN],
            request_body,
        )
        .await
    }

    pub async fn get_coconut_verification_key(
        &self,
    ) -> Result<VerificationKeyResponse, ValidatorAPIError> {
        self.query_validator_api(&[routes::API_VERSION, routes::COCONUT_VERIFICATION_KEY])
            .await
    }
}

// utility function that should solve the double slash problem in validator API forever.
fn create_api_url(base: &Url, segments: PathSegments<'_>) -> Url {
    let mut url = base.clone();
    let mut path_segments = url
        .path_segments_mut()
        .expect("provided validator url does not have a base!");
    for segment in segments {
        let segment = segment.strip_prefix('/').unwrap_or(segment);
        let segment = segment.strip_suffix('/').unwrap_or(segment);

        path_segments.push(segment);
    }
    // I don't understand why compiler couldn't figure out that it's no longer used
    // and can be dropped
    drop(path_segments);

    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_api_path() {
        let base_url: Url = "http://foomp.com".parse().unwrap();

        // works with 1 segment
        assert_eq!(
            "http://foomp.com/foo",
            create_api_url(&base_url, &["foo"]).as_str()
        );

        // works with 2 segments
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo", "bar"]).as_str()
        );

        // works with leading slash
        assert_eq!(
            "http://foomp.com/foo",
            create_api_url(&base_url, &["/foo"]).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["/foo", "bar"]).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo", "/bar"]).as_str()
        );

        // works with trailing slash
        assert_eq!(
            "http://foomp.com/foo",
            create_api_url(&base_url, &["foo/"]).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo/", "bar"]).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo", "bar/"]).as_str()
        );

        // works with both leading and trailing slash
        assert_eq!(
            "http://foomp.com/foo",
            create_api_url(&base_url, &["/foo/"]).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["/foo/", "/bar/"]).as_str()
        );
    }
}
