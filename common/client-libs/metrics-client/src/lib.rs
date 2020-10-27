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

pub mod models;
pub mod requests;

use crate::models::metrics::{MixMetric, PersistedMixMetric};
use crate::requests::metrics_mixes_get::Request as MetricsMixRequest;
use crate::requests::metrics_mixes_post::Request as MetricsMixPost;
use crate::requests::{DirectoryGetRequest, DirectoryPostRequest};

pub struct Config {
    pub base_url: String,
}

impl Config {
    pub fn new(base_url: String) -> Self {
        Config { base_url }
    }
}

pub struct Client {
    base_url: String,
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new(config: Config) -> Client {
        let reqwest_client = reqwest::Client::new();
        Client {
            base_url: config.base_url,
            reqwest_client,
        }
    }

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

    pub async fn post_mix_metrics(&self, metrics: MixMetric) -> reqwest::Result<reqwest::Response> {
        let req = MetricsMixPost::new(&self.base_url, metrics);
        self.post(req).await
    }

    pub async fn get_mix_metrics(&self) -> reqwest::Result<Vec<PersistedMixMetric>> {
        let req = MetricsMixRequest::new(&self.base_url);
        self.get(req).await
    }
}
