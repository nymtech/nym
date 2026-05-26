// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::http::state::helpers::ChainSharedCacheWithTtl;
use crate::support::nyxd::Client;
use nym_api_requests::models::DetailedChainStatus;
use nym_validator_client::nyxd::error::NyxdError;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct ChainStatusCache(ChainSharedCacheWithTtl<DetailedChainStatus>);

impl ChainStatusCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        ChainStatusCache(ChainSharedCacheWithTtl::new(cache_ttl))
    }
}

async fn refresh(client: &Client) -> Result<DetailedChainStatus, NyxdError> {
    // 3. attempt to query the chain for the chain data
    let abci = client.abci_info().await?;
    let block = client
        .block_info(abci.last_block_height.value() as u32)
        .await?;

    Ok(DetailedChainStatus {
        abci: abci.into(),
        latest_block: block.into(),
    })
}

impl ChainStatusCache {
    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<DetailedChainStatus, AxumErrorResponse> {
        self.0.get_or_refresh(client, refresh).await
    }
}
