// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::contract_cache::data::PerformanceContractCacheData;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::refresher::CacheItemProvider;
use crate::support::nyxd::Client;
use async_trait::async_trait;
use nym_validator_client::nyxd::error::NyxdError;

pub struct ContractDataProvider {
    nyxd_client: Client,
    mixnet_contract_cache: NymContractCache,
}

#[async_trait]
impl CacheItemProvider for ContractDataProvider {
    type Item = PerformanceContractCacheData;
    type Error = NyxdError;

    async fn try_refresh(&self) -> Result<Self::Item, Self::Error> {
        self.refresh().await
    }
}

impl ContractDataProvider {
    // pub(crate) fn new(nyxd_client: Client) -> Self {
    //     ContractDataProvider { nyxd_client }
    // }

    async fn refresh(&self) -> Result<PerformanceContractCacheData, NyxdError> {
        todo!()
    }
}
