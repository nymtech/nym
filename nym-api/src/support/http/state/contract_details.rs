// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::nyxd::Client;
use nym_contracts_common::ContractBuildInformation;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, NymContractsProvider, VestingQueryClient,
};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::AccountId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;

type ContractAddress = String;

pub type CachedContractsInfo = HashMap<ContractAddress, CachedContractInfo>;

#[derive(Clone)]
pub struct CachedContractInfo {
    pub(crate) address: Option<AccountId>,
    pub(crate) base: Option<cw2::ContractVersion>,
    pub(crate) detailed: Option<ContractBuildInformation>,
}

impl CachedContractInfo {
    pub fn new(
        address: Option<&AccountId>,
        base: Option<cw2::ContractVersion>,
        detailed: Option<ContractBuildInformation>,
    ) -> Self {
        Self {
            address: address.cloned(),
            base,
            detailed,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ContractDetailsCache {
    cache_ttl: Duration,
    inner: Arc<RwLock<ContractDetailsCacheInner>>,
}

impl ContractDetailsCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        ContractDetailsCache {
            cache_ttl,
            inner: Arc::new(RwLock::new(ContractDetailsCacheInner::new())),
        }
    }
}

struct ContractDetailsCacheInner {
    last_refreshed_at: OffsetDateTime,
    cache_value: CachedContractsInfo,
}

impl ContractDetailsCacheInner {
    pub(crate) fn new() -> Self {
        ContractDetailsCacheInner {
            last_refreshed_at: OffsetDateTime::UNIX_EPOCH,
            cache_value: Default::default(),
        }
    }

    fn is_valid(&self, ttl: Duration) -> bool {
        if self.last_refreshed_at + ttl > OffsetDateTime::now_utc() {
            return true;
        }
        false
    }

    async fn retrieve_nym_contracts_info(
        &self,
        nyxd_client: &Client,
    ) -> Result<CachedContractsInfo, NyxdError> {
        use crate::query_guard;

        let mut updated = HashMap::new();

        let client_guard = nyxd_client.read().await;

        let mixnet = query_guard!(client_guard, mixnet_contract_address());
        let vesting = query_guard!(client_guard, vesting_contract_address());
        let coconut_dkg = query_guard!(client_guard, dkg_contract_address());
        let group = query_guard!(client_guard, group_contract_address());
        let multisig = query_guard!(client_guard, multisig_contract_address());
        let ecash = query_guard!(client_guard, ecash_contract_address());
        let performance = query_guard!(client_guard, performance_contract_address());

        for (address, name) in [
            (mixnet, "nym-mixnet-contract"),
            (vesting, "nym-vesting-contract"),
            (coconut_dkg, "nym-coconut-dkg-contract"),
            (group, "nym-cw4-group-contract"),
            (multisig, "nym-cw3-multisig-contract"),
            (ecash, "nym-ecash-contract"),
            (performance, "nym-performance-contract"),
        ] {
            let (cw2, build_info) = if let Some(address) = address {
                let cw2 = query_guard!(client_guard, try_get_cw2_contract_version(address).await);
                let mut build_info = query_guard!(
                    client_guard,
                    try_get_contract_build_information(address).await
                );

                // for backwards compatibility until we migrate the contracts
                if build_info.is_none() {
                    match name {
                        "nym-mixnet-contract" => {
                            build_info = Some(query_guard!(
                                client_guard,
                                get_mixnet_contract_version().await
                            )?)
                        }
                        "nym-vesting-contract" => {
                            build_info = Some(query_guard!(
                                client_guard,
                                get_vesting_contract_version().await
                            )?)
                        }
                        _ => (),
                    }
                }

                (cw2, build_info)
            } else {
                (None, None)
            };

            updated.insert(
                name.to_string(),
                CachedContractInfo::new(address, cw2, build_info),
            );
        }

        Ok(updated)
    }
}

impl ContractDetailsCache {
    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<CachedContractsInfo, AxumErrorResponse> {
        if let Some(cached) = self.check_cache().await {
            return Ok(cached);
        }

        self.refresh(client).await
    }

    async fn check_cache(&self) -> Option<CachedContractsInfo> {
        let guard = self.inner.read().await;
        if guard.is_valid(self.cache_ttl) {
            return Some(guard.cache_value.clone());
        }
        None
    }

    async fn refresh(&self, client: &Client) -> Result<CachedContractsInfo, AxumErrorResponse> {
        // 1. attempt to get write lock permit
        let mut guard = self.inner.write().await;

        // 2. check if another task hasn't already updated the cache whilst we were waiting for the permit
        if guard.is_valid(self.cache_ttl) {
            return Ok(guard.cache_value.clone());
        }

        // 3. attempt to query the chain for the contracts data
        let updated_values = guard.retrieve_nym_contracts_info(client).await?;
        guard.last_refreshed_at = OffsetDateTime::now_utc();
        guard.cache_value = updated_values.clone();

        Ok(updated_values)
    }
}
