// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nym_api::error::NymAPIError;
use crate::nym_api::routes::{CORE_STATUS_COUNT, SINCE_ARG};
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, VerifyCredentialBody, VerifyCredentialResponse,
};
use nym_api_requests::models::{
    ComputeRewardEstParam, GatewayCoreStatusResponse, GatewayStatusReportResponse,
    GatewayUptimeHistoryResponse, InclusionProbabilityResponse, MixNodeBondAnnotated,
    MixnodeCoreStatusResponse, MixnodeStatusReportResponse, MixnodeStatusResponse,
    MixnodeUptimeHistoryResponse, RequestError, RewardEstimationResponse, StakeSaturationResponse,
    UptimeResponse,
};
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::{GatewayBond, IdentityKeyRef, MixId};
use nym_name_service_common::NameEntry;
use nym_service_provider_directory_common::ServiceInfo;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;

pub mod error;
pub mod routes;

type PathSegments<'a> = &'a [&'a str];
type Params<'a, K, V> = &'a [(K, V)];

const NO_PARAMS: Params<'_, &'_ str, &'_ str> = &[];

#[derive(Clone)]
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

    async fn send_get_request<K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<Response, NymAPIError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = create_api_url(&self.url, path, params);
        Ok(self.reqwest_client.get(url).send().await?)
    }

    async fn query_nym_api<T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, NymAPIError>
    where
        for<'a> T: Deserialize<'a>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let res = self.send_get_request(path, params).await?;
        if res.status().is_success() {
            Ok(res.json().await?)
        } else if res.status() == StatusCode::NOT_FOUND {
            Err(NymAPIError::NotFound)
        } else {
            Err(NymAPIError::GenericRequestFailure(res.text().await?))
        }
    }

    // This works for endpoints returning Result<Json<T>, ErrorResponse>
    async fn query_nym_api_fallible<T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, NymAPIError>
    where
        for<'a> T: Deserialize<'a>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let res = self.send_get_request(path, params).await?;
        let status = res.status();
        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            let request_error: RequestError = res.json().await?;
            Err(NymAPIError::ApiRequestFailure {
                status: status.as_u16(),
                error: request_error,
            })
        }
    }

    async fn post_nym_api<B, T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, NymAPIError>
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
            Err(NymAPIError::GenericRequestFailure(response.text().await?))
        }
    }

    pub async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.query_nym_api(&[routes::API_VERSION, routes::MIXNODES], NO_PARAMS)
            .await
    }

    pub async fn get_mixnodes_detailed(&self) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::DETAILED,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnodes_detailed_unfiltered(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::DETAILED_UNFILTERED,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_gateways(&self) -> Result<Vec<GatewayBond>, NymAPIError> {
        self.query_nym_api(&[routes::API_VERSION, routes::GATEWAYS], NO_PARAMS)
            .await
    }

    pub async fn get_active_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.query_nym_api(
            &[routes::API_VERSION, routes::MIXNODES, routes::ACTIVE],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_active_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::ACTIVE,
                routes::DETAILED,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_rewarded_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NymAPIError> {
        self.query_nym_api(
            &[routes::API_VERSION, routes::MIXNODES, routes::REWARDED],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_report(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeStatusReportResponse, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::REPORT,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_gateway_report(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<GatewayStatusReportResponse, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::GATEWAY,
                identity,
                routes::REPORT,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_history(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeUptimeHistoryResponse, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::HISTORY,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_gateway_history(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<GatewayUptimeHistoryResponse, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::GATEWAY,
                identity,
                routes::HISTORY,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_rewarded_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS,
                routes::MIXNODES,
                routes::REWARDED,
                routes::DETAILED,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_gateway_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<GatewayCoreStatusResponse, NymAPIError> {
        if let Some(since) = since {
            self.query_nym_api(
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
            self.query_nym_api(
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
        mix_id: MixId,
        since: Option<i64>,
    ) -> Result<MixnodeCoreStatusResponse, NymAPIError> {
        if let Some(since) = since {
            self.query_nym_api(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::MIXNODE,
                    &mix_id.to_string(),
                    CORE_STATUS_COUNT,
                ],
                &[(SINCE_ARG, since.to_string())],
            )
            .await
        } else {
            self.query_nym_api(
                &[
                    routes::API_VERSION,
                    routes::STATUS_ROUTES,
                    routes::MIXNODE,
                    &mix_id.to_string(),
                    CORE_STATUS_COUNT,
                ],
                NO_PARAMS,
            )
            .await
        }
    }

    pub async fn get_mixnode_status(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeStatusResponse, NymAPIError> {
        self.query_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::STATUS,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_reward_estimation(
        &self,
        mix_id: MixId,
    ) -> Result<RewardEstimationResponse, NymAPIError> {
        self.query_nym_api_fallible(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::REWARD_ESTIMATION,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn compute_mixnode_reward_estimation(
        &self,
        mix_id: MixId,
        request_body: &ComputeRewardEstParam,
    ) -> Result<RewardEstimationResponse, NymAPIError> {
        self.post_nym_api(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::COMPUTE_REWARD_ESTIMATION,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    pub async fn get_mixnode_stake_saturation(
        &self,
        mix_id: MixId,
    ) -> Result<StakeSaturationResponse, NymAPIError> {
        self.query_nym_api_fallible(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::STAKE_SATURATION,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_inclusion_probability(
        &self,
        mix_id: MixId,
    ) -> Result<InclusionProbabilityResponse, NymAPIError> {
        self.query_nym_api_fallible(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::INCLUSION_CHANCE,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn get_mixnode_avg_uptime(
        &self,
        mix_id: MixId,
    ) -> Result<UptimeResponse, NymAPIError> {
        self.query_nym_api_fallible(
            &[
                routes::API_VERSION,
                routes::STATUS_ROUTES,
                routes::MIXNODE,
                &mix_id.to_string(),
                routes::AVG_UPTIME,
            ],
            NO_PARAMS,
        )
        .await
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, NymAPIError> {
        self.post_nym_api(
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

    pub async fn verify_bandwidth_credential(
        &self,
        request_body: &VerifyCredentialBody,
    ) -> Result<VerifyCredentialResponse, NymAPIError> {
        self.post_nym_api(
            &[
                routes::API_VERSION,
                routes::COCONUT_ROUTES,
                routes::BANDWIDTH,
                routes::COCONUT_VERIFY_BANDWIDTH_CREDENTIAL,
            ],
            NO_PARAMS,
            request_body,
        )
        .await
    }

    pub async fn get_service_providers(&self) -> Result<Vec<ServiceInfo>, NymAPIError> {
        self.query_nym_api(&[routes::API_VERSION, routes::SERVICE_PROVIDERS], NO_PARAMS)
            .await
    }

    pub async fn get_registered_names(&self) -> Result<Vec<NameEntry>, NymAPIError> {
        self.query_nym_api(&[routes::API_VERSION, routes::REGISTERED_NAMES], NO_PARAMS)
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
