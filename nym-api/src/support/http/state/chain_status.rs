// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::nyxd::Client;
use nym_api_requests::models::DetailedChainStatus;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;

#[derive(Clone)]
pub(crate) struct ChainStatusCache {
    cache_ttl: Duration,
    inner: Arc<RwLock<Option<ChainStatusCacheInner>>>,
}

impl ChainStatusCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        ChainStatusCache {
            cache_ttl,
            inner: Arc::new(Default::default()),
        }
    }
}

struct ChainStatusCacheInner {
    last_refreshed_at: OffsetDateTime,
    cache_value: DetailedChainStatus,
}

impl ChainStatusCacheInner {
    fn is_valid(&self, ttl: Duration) -> bool {
        if self.last_refreshed_at + ttl > OffsetDateTime::now_utc() {
            return true;
        }
        false
    }
}

impl ChainStatusCache {
    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<DetailedChainStatus, AxumErrorResponse> {
        if let Some(cached) = self.check_cache().await {
            return Ok(cached);
        }

        self.refresh(client).await
    }

    async fn check_cache(&self) -> Option<DetailedChainStatus> {
        let guard = self.inner.read().await;
        let inner = guard.as_ref()?;
        if inner.is_valid(self.cache_ttl) {
            return Some(inner.cache_value.clone());
        }
        None
    }

    async fn refresh(&self, client: &Client) -> Result<DetailedChainStatus, AxumErrorResponse> {
        // 1. attempt to get write lock permit
        let mut guard = self.inner.write().await;

        // 2. check if another task hasn't already updated the cache whilst we were waiting for the permit
        if let Some(cached) = guard.as_ref() {
            if cached.is_valid(self.cache_ttl) {
                return Ok(cached.cache_value.clone());
            }
        }

        // 3. attempt to query the chain for the chain data
        let abci = client.abci_info().await?;
        let block = client
            .block_info(abci.last_block_height.value() as u32)
            .await?;

        let status = DetailedChainStatus {
            abci: abci.into(),
            latest_block: block.into(),
        };

        *guard = Some(ChainStatusCacheInner {
            last_refreshed_at: OffsetDateTime::now_utc(),
            cache_value: status.clone(),
        });

        Ok(status)
    }
}
