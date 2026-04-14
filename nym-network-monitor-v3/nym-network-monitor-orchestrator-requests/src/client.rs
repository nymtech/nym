// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::{
    AgentAnnounceRequest, AgentPortRequest, AgentPortRequestResponse, TestRunAssignment,
};
use crate::routes::v1::agent::{
    announce_absolute, port_request_absolute, request_testrun_absolute,
};
pub use nym_http_api_client::Client;
use nym_http_api_client::{ApiClient, HttpClientError, NO_PARAMS, Url, parse_response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroizing;

pub struct OrchestratorClient {
    inner: Client,
    bearer_token: Arc<Zeroizing<String>>,
}

impl OrchestratorClient {
    pub fn new(base_url: Url, bearer_token: String) -> Result<Self, HttpClientError> {
        Ok(OrchestratorClient {
            inner: Client::builder(base_url)?
                .no_hickory_dns()
                .with_user_agent(format!(
                    "nym-network-monitor-orchestrator-requests/{}",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()?,
            bearer_token: Arc::new(Zeroizing::new(bearer_token)),
        })
    }

    async fn post_with_auth<B, T>(&self, path: &str, json_body: &B) -> Result<T, HttpClientError>
    where
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
    {
        let res = self
            .inner
            .create_post_request(path, NO_PARAMS, json_body)?
            .bearer_auth(self.bearer_token.as_str())
            .send()
            .await?;

        parse_response(res, false).await
    }

    async fn get_with_auth<T>(&self, path: &str) -> Result<T, HttpClientError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let res = self
            .inner
            .create_get_request(path, NO_PARAMS)?
            .bearer_auth(self.bearer_token.as_str())
            .send()
            .await?;

        parse_response(res, false).await
    }

    pub async fn get_mix_port_assignment(
        &self,
        body: &AgentPortRequest,
    ) -> Result<AgentPortRequestResponse, HttpClientError> {
        self.post_with_auth(&port_request_absolute(), body).await
    }

    pub async fn announce_agent(&self, body: &AgentAnnounceRequest) -> Result<(), HttpClientError> {
        self.post_with_auth(&announce_absolute(), body).await
    }

    pub async fn request_work_assignment(&self) -> Result<TestRunAssignment, HttpClientError> {
        self.get_with_auth(&request_testrun_absolute()).await
    }
}
