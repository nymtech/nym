// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_performance::contract_cache::data::PerformanceContractCacheData;
use crate::support::caching::refresher::CacheItemProvider;
use crate::support::nyxd::Client;
use async_trait::async_trait;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::contract_traits::performance_query_client::LastSubmission;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::HashMap;

pub struct Config {
    max_epochs_to_keep: usize,
}

pub struct PerformanceContractDataProvider {
    nyxd_client: Client,
    mixnet_contract_cache: MixnetContractCache,
    last_submission: LastSubmission,
}

#[async_trait]
impl CacheItemProvider for PerformanceContractDataProvider {
    type Item = PerformanceContractCacheData;
    type Error = NyxdError;

    async fn wait_until_ready(&self) {
        self.mixnet_contract_cache
            .naive_wait_for_initial_values()
            .await
    }

    async fn try_refresh(&self) -> Result<Option<Self::Item>, Self::Error> {
        self.refresh().await
    }
}

impl PerformanceContractDataProvider {
    // pub(crate) fn new(nyxd_client: Client) -> Self {
    //     ContractDataProvider { nyxd_client }
    // }

    fn get_epoch_performance(&self) -> Result<HashMap<NodeId, Percent>, NyxdError> {
        todo!()
    }

    async fn refresh(&self) -> Result<Option<PerformanceContractCacheData>, NyxdError> {
        let last_submitted = self
            .nyxd_client
            .get_last_performance_contract_submission()
            .await?;

        // no updates
        if last_submitted == self.last_submission {
            return Ok(None);
        }

        // TODO: it's quite wasteful to clone the whole thing

        // SAFETY: refresher is not started until the mixnet contract cache had been initialised
        #[allow(clippy::unwrap_used)]
        let current_epoch = self.mixnet_contract_cache.current_interval().await.unwrap();

        // get data on the current epoch

        todo!()
    }
}
