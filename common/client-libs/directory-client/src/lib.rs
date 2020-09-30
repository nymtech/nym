// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::requests::health_check_get::Request as HealthCheckRequest;
use crate::requests::metrics_mixes_get::Request as MetricsMixRequest;
use crate::requests::metrics_mixes_post::Request as MetricsMixPost;
use crate::requests::mix_mining_status_post::Request as MixMiningStatusPost;
use crate::requests::presence_coconodes_post::Request as PresenceCocoNodesPost;
use crate::requests::presence_gateways_post::Request as PresenceGatewayPost;
use crate::requests::presence_mixnodes_post::Request as PresenceMixNodesPost;
use crate::requests::presence_providers_post::Request as PresenceProvidersPost;
use crate::requests::presence_topology_get::Request as PresenceTopologyRequest;
use directory_client_models::metrics::{MixMetric, PersistedMixMetric};
use directory_client_models::presence::{
    coconodes::CocoPresence, gateways::GatewayPresence, mixnodes::MixNodePresence,
    providers::MixProviderPresence,
};
use mixmining::MixStatus;
use requests::{health_check_get::HealthCheckResponse, DirectoryGetRequest, DirectoryPostRequest};

pub use directory_client_models::{
    metrics, mixmining,
    presence::{self, Topology},
};

pub mod requests;

pub struct Config {
    pub base_url: String,
}

impl Config {
    pub fn new(base_url: String) -> Self {
        Config { base_url }
    }
}

pub trait DirectoryClient {
    fn new(config: Config) -> Self;
}

pub struct Client {
    base_url: String,
    reqwest_client: reqwest::Client,
}

impl DirectoryClient for Client {
    fn new(config: Config) -> Client {
        let reqwest_client = reqwest::Client::new();
        Client {
            base_url: config.base_url,
            reqwest_client,
        }
    }
}

impl Client {
    async fn post<R: DirectoryPostRequest>(
        &self,
        request: R,
    ) -> reqwest::Result<reqwest::Response> {
        self.reqwest_client
            .post(&request.url())
            .json(request.json_payload())
            .send()
            .await
    }

    async fn get<R: DirectoryGetRequest>(&self, request: R) -> reqwest::Result<R::JSONResponse> {
        self.reqwest_client
            .get(&request.url())
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_healthcheck(&self) -> reqwest::Result<HealthCheckResponse> {
        let req = HealthCheckRequest::new(&self.base_url);
        self.get(req).await
    }

    pub async fn post_mix_metrics(&self, metrics: MixMetric) -> reqwest::Result<reqwest::Response> {
        let req = MetricsMixPost::new(&self.base_url, metrics);
        self.post(req).await
    }

    pub async fn get_mix_metrics(&self) -> reqwest::Result<Vec<PersistedMixMetric>> {
        let req = MetricsMixRequest::new(&self.base_url);
        self.get(req).await
    }

    pub async fn post_coconode_presence(
        &self,
        presence: CocoPresence,
    ) -> reqwest::Result<reqwest::Response> {
        let req = PresenceCocoNodesPost::new(&self.base_url, presence);
        self.post(req).await
    }

    pub async fn post_gateway_presence(
        &self,
        presence: GatewayPresence,
    ) -> reqwest::Result<reqwest::Response> {
        let req = PresenceGatewayPost::new(&self.base_url, presence);
        self.post(req).await
    }

    pub async fn post_mixnode_presence(
        &self,
        presence: MixNodePresence,
    ) -> reqwest::Result<reqwest::Response> {
        let req = PresenceMixNodesPost::new(&self.base_url, presence);
        self.post(req).await
    }

    pub async fn post_mixmining_status(
        &self,
        status: MixStatus,
    ) -> reqwest::Result<reqwest::Response> {
        let req = MixMiningStatusPost::new(&self.base_url, status);
        self.post(req).await
    }

    // this should be soft-deprecated as the whole concept of provider will
    // be removed in the next topology rework
    pub async fn post_provider_presence(
        &self,
        presence: MixProviderPresence,
    ) -> reqwest::Result<reqwest::Response> {
        let req = PresenceProvidersPost::new(&self.base_url, presence);
        self.post(req).await
    }

    pub async fn get_topology(&self) -> reqwest::Result<Topology> {
        let req = PresenceTopologyRequest::new(&self.base_url);
        self.get(req).await
    }
}

#[cfg(test)]
pub(crate) fn client_test_fixture(base_url: &str) -> Client {
    Client {
        base_url: base_url.to_string(),
        reqwest_client: reqwest::Client::new(),
    }
}
