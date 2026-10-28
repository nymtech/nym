// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::checkpoint::SignedCheckpoint;
use crate::anchor::checkpoint::fetcher::CheckpointFetcher;
use async_trait::async_trait;
use nym_http_api_client::{ApiClient, HttpClientError, UserAgent};
use std::time::Duration;

#[async_trait]
impl CheckpointFetcher for nym_http_api_client::Client {
    type Error = HttpClientError;

    async fn fetch(&self) -> Result<SignedCheckpoint, Self::Error> {
        self.get_json_from("/").await
    }
}

pub fn basic_checkpoint_fetcher(
    url: &str,
    use_hickory: bool,
    user_agent: Option<UserAgent>,
) -> Result<nym_http_api_client::Client, HttpClientError> {
    let builder = nym_http_api_client::ClientBuilder::new(url)?
        .with_user_agent(user_agent.unwrap_or_else(|| nym_http_api_client::generate_user_agent!()))
        .with_timeout(Duration::from_secs(5));

    if use_hickory {
        builder.build()
    } else {
        builder.no_hickory_dns().build()
    }
}
