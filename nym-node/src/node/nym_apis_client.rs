// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymNodeError;
use crate::node::NymNode;
use futures::{stream, StreamExt};
use nym_crypto::asymmetric::ed25519;
use nym_http_api_client::Client;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::NodeRefreshBody;
use nym_validator_client::nym_api::error::NymAPIError;
use nym_validator_client::NymApiClient;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::warn;
use url::Url;

#[derive(Clone)]
pub struct NymApisClient {
    inner: Arc<RwLock<InnerClient>>,
}

struct InnerClient {
    active_client: NymApiClient,
    available_urls: Vec<Url>,
    currently_used_api: usize,
}

impl NymApisClient {
    pub(crate) fn new(nym_apis: &[Url]) -> Result<Self, NymNodeError> {
        if nym_apis.is_empty() {
            return Err(NymNodeError::NoNymApiUrls);
        }

        let mut urls = nym_apis.to_vec();
        urls.shuffle(&mut thread_rng());

        let active_client = nym_http_api_client::Client::builder(urls[0].clone())?
            .no_hickory_dns()
            .with_user_agent(NymNode::user_agent())
            .build()?;

        Ok(NymApisClient {
            inner: Arc::new(RwLock::new(InnerClient {
                active_client: NymApiClient {
                    nym_api: active_client,
                },
                available_urls: urls,
                currently_used_api: 0,
            })),
        })
    }

    async fn use_next_endpoint(&mut self) {
        let mut guard = self.inner.write().await;
        if guard.available_urls.len() == 1 {
            return;
        }

        let next_index = (guard.currently_used_api + 1) % guard.available_urls.len();
        let next = guard.available_urls[next_index].clone();
        guard.currently_used_api = next_index;
        guard.active_client.change_nym_api(next)
    }

    pub(crate) async fn query_exhaustively<R, T>(
        &self,
        req: R,
        timeout_duration: Duration,
    ) -> Result<T, NymNodeError>
    where
        R: AsyncFn(Client) -> Result<T, NymAPIError>,
    {
        self.inner
            .read()
            .await
            .query_exhaustively(req, timeout_duration)
            .await
    }

    pub(crate) async fn broadcast_force_refresh(&self, private_key: &ed25519::PrivateKey) {
        self.inner
            .read()
            .await
            .broadcast_force_refresh(private_key)
            .await;
    }

    pub(crate) async fn broadcast_key_rotation(&self) {
        self.inner.read().await.broadcast_key_rotation().await;
    }
}

impl InnerClient {
    // currently there are no cases without json body, but for those we'd just need to slightly adjust the signature
    async fn broadcast<B, R>(&self, request_body: &B, req: R, timeout_duration: Duration)
    where
        R: AsyncFn(Client, &B) -> Result<(), NymAPIError>,
    {
        let broadcast_fut =
            stream::iter(self.available_urls.clone()).for_each_concurrent(None, |url| {
                let nym_api = self.active_client.nym_api.clone_with_new_url(url.clone());
                let req_fut = req(nym_api, request_body);
                async move {
                    if let Err(err) = req_fut.await {
                        warn!("broadcast request to {url} failed: {err}")
                    }
                }
            });

        if timeout(timeout_duration, broadcast_fut).await.is_err() {
            warn!("timed out while attempting to broadcast data to known nym apis")
        }
    }

    pub(crate) async fn query_exhaustively<R, T>(
        &self,
        req: R,
        timeout_duration: Duration,
    ) -> Result<T, NymNodeError>
    where
        R: AsyncFn(Client) -> Result<T, NymAPIError>,
    {
        // this is DESIGNED to query sequentially (but exhaustively) and not to try to send queries to ALL apis at once
        // and check which resolves first
        for url in &self.available_urls {
            let nym_api = self.active_client.nym_api.clone_with_new_url(url.clone());
            match timeout(timeout_duration, req(nym_api)).await {
                Ok(Ok(res)) => return Ok(res),
                Ok(Err(err)) => {
                    warn!("failed to resolve query for {url}: {err}")
                }
                Err(_timeout) => {
                    warn!("timed out while attempting to query {url}")
                }
            }
        }

        Err(NymNodeError::NymApisExhausted)
    }

    pub(crate) async fn broadcast_force_refresh(&self, private_key: &ed25519::PrivateKey) {
        let request = NodeRefreshBody::new(private_key);

        self.broadcast(
            &request,
            async |client, request| client.force_refresh_describe_cache(request).await,
            Duration::from_secs(10),
        )
        .await;
    }

    pub(crate) async fn broadcast_key_rotation(&self) {
        todo!()
    }
}

impl AsRef<NymApiClient> for InnerClient {
    fn as_ref(&self) -> &NymApiClient {
        &self.active_client
    }
}
