// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::{
    AgentAnnounceRequest, AgentAnnounceResponse, WorkAssignmentRequest, WorkAssignmentResponse,
    WorkResultSubmissionRequest, WorkResultSubmissionResponse,
};
use crate::routes::v1::agent::{
    announce_absolute, request_work_assignment_absolute, submit_work_result_absolute,
};
pub use nym_http_api_client::Client;
use nym_http_api_client::{ApiClient, HttpClientError, NO_PARAMS, Url, parse_response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroizing;

/// HTTP client for communicating with the network monitor orchestrator API.
/// All requests are authenticated with a bearer token.
pub struct OrchestratorClient {
    inner: Client,
    bearer_token: Arc<Zeroizing<String>>,
}

impl OrchestratorClient {
    /// Creates a new client targeting `base_url`, storing the bearer token in a
    /// zeroizing container.
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

    /// Sends an authenticated POST request with a JSON body and deserialises the response.
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

    /// Announces this agent's details to the orchestrator, which forwards them
    /// to the smart contract so network nodes can whitelist the agent.
    pub async fn announce_agent(
        &self,
        body: &AgentAnnounceRequest,
    ) -> Result<AgentAnnounceResponse, HttpClientError> {
        self.post_with_auth(&announce_absolute(), body).await
    }

    /// Asks the orchestrator for the next work to execute. Returns `None`
    /// inside the assignment if no work is currently available.
    pub async fn request_work_assignment(
        &self,
        body: &WorkAssignmentRequest,
    ) -> Result<WorkAssignmentResponse, HttpClientError> {
        self.post_with_auth(&request_work_assignment_absolute(), body)
            .await
    }

    /// Submits the result of assigned work back to the orchestrator for storage.
    pub async fn submit_work_result(
        &self,
        body: &WorkResultSubmissionRequest,
    ) -> Result<WorkResultSubmissionResponse, HttpClientError> {
        self.post_with_auth(&submit_work_result_absolute(), body)
            .await
    }
}
