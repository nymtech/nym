// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::http::state::helpers::ChainSharedCacheWithTtl;
use crate::support::nyxd::Client;
use nym_contracts_common::ContractBuildInformation;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, NymContractsProvider, VestingQueryClient,
};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::AccountId;
use std::collections::HashMap;
use std::time::Duration;

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
pub(crate) struct ContractDetailsCache(ChainSharedCacheWithTtl<CachedContractsInfo>);

async fn refresh(nyxd_client: &Client) -> Result<CachedContractsInfo, NyxdError> {
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
    let node_families = query_guard!(client_guard, node_families_contract_address());

    for (address, name) in [
        (mixnet, "nym-mixnet-contract"),
        (vesting, "nym-vesting-contract"),
        (coconut_dkg, "nym-coconut-dkg-contract"),
        (group, "nym-cw4-group-contract"),
        (multisig, "nym-cw3-multisig-contract"),
        (ecash, "nym-ecash-contract"),
        (performance, "nym-performance-contract"),
        (node_families, "nym-node-families-contract"),
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

impl ContractDetailsCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        ContractDetailsCache(ChainSharedCacheWithTtl::new(cache_ttl))
    }

    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<CachedContractsInfo, AxumErrorResponse> {
        self.0.get_or_refresh(client, refresh).await
    }
}
