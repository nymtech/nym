// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::validator_api::error::ValidatorAPIError;
use crate::validator_api::routes::{CORE_STATUS_COUNT, SINCE_ARG};
use coconut_interface::{BlindSignRequestBody, BlindedSignatureResponse, VerificationKeyResponse};
use mixnet_contract_common::{GatewayBond, IdentityKeyRef, MixNodeBond};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use validator_api_requests::models::{
    CoreNodeStatusResponse, InclusionProbabilityResponse, MixnodeStatusResponse,
    RewardEstimationResponse, StakeSaturationResponse, UptimeResponse,
};

pub mod error;
pub mod routes;

type PathSegments<'a> = &'a [&'a str];
type Params<'a, K, V> = &'a [(K, V)];

const NO_PARAMS: Params<'_, &'_ str, &'_ str> = &[];

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

    pub fn current_url(&self) -> &Url {
        &self.url
    }

    async fn query_validator_api<T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, ValidatorAPIError>
    where
        for<'a> T: Deserialize<'a>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = create_api_url(&self.url, path, params);
        Ok(self.reqwest_client.get(url).send().await?.json().await?)
    }

    async fn post_validator_api<B, T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, ValidatorAPIError>
    where
        B: Serialize + ?Sized,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = create_api_url(&self.url, path, params);
        let response = self.reqwest_client.post(url).json(json_body).send().await?;
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(ValidatorAPIError::GenericRequestFailure(
                response.text().await?,
            ))
        }
    }

    pub async fn get_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorAPIError> {
        self.query_validator_api(&[routes::API_VERSION, routes::MIXNODES], NO_PARAMS)
            .await
    }

    pub async fn get_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorAPIError> {
        self.query_validator_api(&[routes::API_VERSION, routes::GATEWAYS], NO_PARAMS)
            .await
    }

    pub async fn get_active_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorAPIError> {
        self.query_validator_api(
            &[routes::API_VERSION, routes::MIXNODES, routes::ACTIVE],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_rewarded_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorAPIError> {
        self.query_validator_api(
            &[routes::API_VERSION, routes::MIXNODES, routes::REWARDED],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_probs_mixnode_rewarded(
        &self,
        mixnode_id: &str,
    ) -> Result<HashMap<String, f32>, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::MIXNODES,
                routes::REWARDED,
                routes::INCLUSION_CHANCE,
                mixnode_id,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_gateway_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<CoreNodeStatusResponse, ValidatorAPIError> {
        if let Some(since) = since {
            self.query_validator_api(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::GATEWAY,
                    identity,
                    CORE_STATUS_COUNT,
                ],
                &[(SINCE_ARG, since.to_string())],
            )
            .await
        } else {
            self.query_validator_api(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::GATEWAY,
                    identity,
                ],
                NO_PARAMS,
            )
            .await
        }
    }

    pub async fn get_mixnode_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<CoreNodeStatusResponse, ValidatorAPIError> {
        if let Some(since) = since {
            self.query_validator_api(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::MIXNODE,
                    identity,
                    CORE_STATUS_COUNT,
                ],
                &[(SINCE_ARG, since.to_string())],
            )
            .await
        } else {
            self.query_validator_api(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::MIXNODE,
                    identity,
                ],
                NO_PARAMS,
            )
            .await
        }
    }

    pub async fn get_mixnode_status(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<MixnodeStatusResponse, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                identity,
                routes::STATUS,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_reward_estimation(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<RewardEstimationResponse, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                identity,
                routes::REWARD_ESTIMATION,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_stake_saturation(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<StakeSaturationResponse, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                identity,
                routes::STAKE_SATURATION,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_inclusion_probability(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<InclusionProbabilityResponse, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                identity,
                routes::INCLUSION_CHANCE,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_avg_uptime(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<UptimeResponse, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                identity,
                routes::AVG_UPTIME,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_avg_uptimes(&self) -> Result<Vec<UptimeResponse>, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODES,
                routes::AVG_UPTIME,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorAPIError> {
        self.post_validator_api(
            &[
                routes::API_VERSION,
                routes::COCONUT_ROUTES,
                routes::BANDWIDTH,
                routes::COCONUT_BLIND_SIGN,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    pub async fn partial_bandwidth_credential(
        &self,
        request_body: &str,
    ) -> Result<BlindedSignatureResponse, ValidatorAPIError> {
        self.post_validator_api(
            &[
                routes::API_VERSION,
                routes::COCONUT_ROUTES,
                routes::BANDWIDTH,
                routes::COCONUT_PARTIAL_BANDWIDTH_CREDENTIAL,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    pub async fn get_coconut_verification_key(
        &self,
    ) -> Result<VerificationKeyResponse, ValidatorAPIError> {
        self.query_validator_api(
            &[
                routes::API_VERSION,
                routes::COCONUT_ROUTES,
                routes::BANDWIDTH,
                routes::COCONUT_VERIFICATION_KEY,
            ],
            NO_PARAMS,
        )
        .await
    }
}

// utility function that should solve the double slash problem in validator API forever.
fn create_api_url<K: AsRef<str>, V: AsRef<str>>(
    base: &Url,
    segments: PathSegments<'_>,
    params: Params<'_, K, V>,
) -> Url {
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

    if !params.is_empty() {
        url.query_pairs_mut().extend_pairs(params);
    }

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
            create_api_url(&base_url, &["foo"], NO_PARAMS).as_str()
        );

        // works with 2 segments
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo", "bar"], NO_PARAMS).as_str()
        );

        // works with leading slash
        assert_eq!(
            "http://foomp.com/foo",
            create_api_url(&base_url, &["/foo"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["/foo", "bar"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo", "/bar"], NO_PARAMS).as_str()
        );

        // works with trailing slash
        assert_eq!(
            "http://foomp.com/foo",
            create_api_url(&base_url, &["foo/"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo/", "bar"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["foo", "bar/"], NO_PARAMS).as_str()
        );

        // works with both leading and trailing slash
        assert_eq!(
            "http://foomp.com/foo",
            create_api_url(&base_url, &["/foo/"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            create_api_url(&base_url, &["/foo/", "/bar/"], NO_PARAMS).as_str()
        );

        // adds params
        assert_eq!(
            "http://foomp.com/foo/bar?foomp=baz",
            create_api_url(&base_url, &["foo", "bar"], &[("foomp", "baz")]).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar?arg1=val1&arg2=val2",
            create_api_url(
                &base_url,
                &["/foo/", "/bar/"],
                &[("arg1", "val1"), ("arg2", "val2")]
            )
            .as_str()
        );
    }
}
