// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_performance::contract_cache::data::{
    PerformanceContractCacheData, PerformanceContractEpochCacheData,
};
use crate::support::caching::refresher::CacheItemProvider;
use crate::support::nyxd::Client;
use async_trait::async_trait;
use nym_validator_client::nyxd::contract_traits::performance_query_client::LastSubmission;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::BTreeMap;

pub struct PerformanceContractDataProvider {
    nyxd_client: Client,
    mixnet_contract_cache: MixnetContractCache,
    last_submission: Option<LastSubmission>,
}

pub(crate) fn refresher_update_fn(
    main_cache: &mut PerformanceContractCacheData,
    update: PerformanceContractEpochCacheData,
    values_to_retain: usize,
) {
    main_cache.update(update, values_to_retain);
}

#[async_trait]
impl CacheItemProvider for PerformanceContractDataProvider {
    type Item = PerformanceContractEpochCacheData;
    type Error = NyxdError;

    async fn wait_until_ready(&self) {
        self.mixnet_contract_cache
            .naive_wait_for_initial_values()
            .await
    }

    async fn try_refresh(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        self.refresh().await
    }
}

impl PerformanceContractDataProvider {
    pub(crate) fn new(nyxd_client: Client, mixnet_contract_cache: MixnetContractCache) -> Self {
        PerformanceContractDataProvider {
            nyxd_client,
            mixnet_contract_cache,
            last_submission: None,
        }
    }

    pub(crate) async fn cache_has_values(&self) -> bool {
        let Ok(last_submitted) = self
            .nyxd_client
            .get_last_performance_contract_submission()
            .await
        else {
            return false;
        };
        last_submitted.data.is_some()
    }

    pub(crate) async fn provide_initial_warmed_up_cache(
        &mut self,
        values_to_keep: usize,
    ) -> Result<PerformanceContractCacheData, NyxdError> {
        let last_submitted = self
            .nyxd_client
            .get_last_performance_contract_submission()
            .await?;

        self.mixnet_contract_cache
            .naive_wait_for_initial_values()
            .await;

        // SAFETY: we just waited for cache to be available
        #[allow(clippy::unwrap_used)]
        let current_epoch = self
            .mixnet_contract_cache
            .current_interval()
            .await
            .unwrap()
            .current_epoch_absolute_id();

        let last = current_epoch.saturating_sub(values_to_keep as u32);

        let mut epoch_performance = BTreeMap::default();
        for epoch in current_epoch..last {
            let performance = self.nyxd_client.get_full_epoch_performance(epoch).await?;
            let per_epoch_performance =
                PerformanceContractEpochCacheData::from_node_performance(performance, epoch);
            epoch_performance.insert(epoch, per_epoch_performance);
        }

        self.last_submission = Some(last_submitted);

        Ok(PerformanceContractCacheData { epoch_performance })
    }

    async fn refresh(&mut self) -> Result<Option<PerformanceContractEpochCacheData>, NyxdError> {
        let last_submitted = self
            .nyxd_client
            .get_last_performance_contract_submission()
            .await?;

        // no updates
        if let Some(prior_submission) = &self.last_submission {
            if prior_submission == &last_submitted {
                return Ok(None);
            }
        }

        // SAFETY: refresher is not started until the mixnet contract cache had been initialised
        #[allow(clippy::unwrap_used)]
        let current_epoch = self
            .mixnet_contract_cache
            .current_interval()
            .await
            .unwrap()
            .current_epoch_absolute_id();

        let performance = self
            .nyxd_client
            .get_full_epoch_performance(current_epoch)
            .await?;

        self.last_submission = Some(last_submitted);

        Ok(Some(
            PerformanceContractEpochCacheData::from_node_performance(performance, current_epoch),
        ))
    }
}
