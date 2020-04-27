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

use crate::requests::health_check_get::{HealthCheckRequester, Request as HealthCheckRequest};
use crate::requests::metrics_mixes_get::{MetricsMixRequester, Request as MetricsMixRequest};
use crate::requests::metrics_mixes_post::{MetricsMixPoster, Request as MetricsMixPost};
use crate::requests::presence_coconodes_post::{
    PresenceCocoNodesPoster, Request as PresenceCocoNodesPost,
};
use crate::requests::presence_gateways_post:: {
    PresenceGatewayPoster, Request as PresenceGatewayPost,
};
use crate::requests::presence_mixnodes_post::{
    PresenceMixNodesPoster, Request as PresenceMixNodesPost,
};
use crate::requests::presence_providers_post::{
    PresenceMixProviderPoster, Request as PresenceProvidersPost,
};
use crate::requests::presence_topology_get::{
    PresenceTopologyGetRequester, Request as PresenceTopologyRequest,
};

pub mod metrics;
pub mod presence;
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
    pub health_check: HealthCheckRequest,
    pub metrics_mixes: MetricsMixRequest,
    pub metrics_post: MetricsMixPost,
    pub presence_coconodes_post: PresenceCocoNodesPost,
    pub presence_gateway_post: PresenceGatewayPost,
    pub presence_mix_nodes_post: PresenceMixNodesPost,
    pub presence_providers_post: PresenceProvidersPost,
    pub presence_topology: PresenceTopologyRequest,
}

impl DirectoryClient for Client {
    fn new(config: Config) -> Client {
        let health_check = HealthCheckRequest::new(config.base_url.clone());
        let metrics_mixes = MetricsMixRequest::new(config.base_url.clone());
        let metrics_post = MetricsMixPost::new(config.base_url.clone());
        let presence_topology = PresenceTopologyRequest::new(config.base_url.clone());
        let presence_coconodes_post = PresenceCocoNodesPost::new(config.base_url.clone());
        let presence_gateway_post = PresenceGatewayPost::new(config.base_url.clone());
        let presence_mix_nodes_post = PresenceMixNodesPost::new(config.base_url.clone());
        let presence_providers_post = PresenceProvidersPost::new(config.base_url);
        Client {
            health_check,
            metrics_mixes,
            metrics_post,
            presence_coconodes_post,
            presence_gateway_post,
            presence_mix_nodes_post,
            presence_providers_post,
            presence_topology,
        }
    }
}
