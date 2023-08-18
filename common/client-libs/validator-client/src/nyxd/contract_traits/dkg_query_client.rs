// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_coconut_dkg_common::dealer::{
    ContractDealing, DealerDetailsResponse, PagedDealerResponse, PagedDealingsResponse,
};
use nym_coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use nym_coconut_dkg_common::types::{DealerDetails, Epoch, EpochId, InitialReplacementData};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, PagedVKSharesResponse};
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait DkgQueryClient {
    async fn query_dkg_contract<T>(&self, query: DkgQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_current_epoch(&self) -> Result<Epoch, NyxdError> {
        let request = DkgQueryMsg::GetCurrentEpochState {};
        self.query_dkg_contract(request).await
    }
    async fn get_current_epoch_threshold(&self) -> Result<Option<u64>, NyxdError> {
        let request = DkgQueryMsg::GetCurrentEpochThreshold {};
        self.query_dkg_contract(request).await
    }

    async fn get_initial_dealers(&self) -> Result<Option<InitialReplacementData>, NyxdError> {
        let request = DkgQueryMsg::GetInitialDealers {};
        self.query_dkg_contract(request).await
    }

    async fn get_dealer_details(
        &self,
        address: &AccountId,
    ) -> Result<DealerDetailsResponse, NyxdError> {
        let request = DkgQueryMsg::GetDealerDetails {
            dealer_address: address.to_string(),
        };
        self.query_dkg_contract(request).await
    }

    async fn get_current_dealers_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NyxdError> {
        let request = DkgQueryMsg::GetCurrentDealers { start_after, limit };
        self.query_dkg_contract(request).await
    }

    async fn get_past_dealers_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NyxdError> {
        let request = DkgQueryMsg::GetPastDealers { start_after, limit };
        self.query_dkg_contract(request).await
    }

    async fn get_dealings_paged(
        &self,
        idx: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedDealingsResponse, NyxdError> {
        let request = DkgQueryMsg::GetDealing {
            idx,
            limit,
            start_after,
        };
        self.query_dkg_contract(request).await
    }

    async fn get_vk_shares_paged(
        &self,
        epoch_id: EpochId,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedVKSharesResponse, NyxdError> {
        let request = DkgQueryMsg::GetVerificationKeys {
            epoch_id,
            limit,
            start_after,
        };
        self.query_dkg_contract(request).await
    }
}

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedDkgQueryClient: DkgQueryClient {
    async fn get_all_current_dealers(&self) -> Result<Vec<DealerDetails>, NyxdError> {
        collect_paged!(self, get_current_dealers_paged, dealers)
    }

    async fn get_all_past_dealers(&self) -> Result<Vec<DealerDetails>, NyxdError> {
        collect_paged!(self, get_past_dealers_paged, dealers)
    }

    async fn get_all_epoch_dealings(&self, idx: u64) -> Result<Vec<ContractDealing>, NyxdError> {
        collect_paged!(self, get_dealings_paged, dealings, idx)
    }

    async fn get_all_verification_key_shares(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<ContractVKShare>, NyxdError> {
        collect_paged!(self, get_vk_shares_paged, shares, epoch_id)
    }
}

#[async_trait]
impl<T> PagedDkgQueryClient for T where T: DkgQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> DkgQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_dkg_contract<T>(&self, query: DkgQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let dkg_contract_address = &self
            .dkg_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("dkg contract"))?;
        self.query_contract_smart(dkg_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: DkgQueryClient + Send + Sync>(
        client: C,
        msg: DkgQueryMsg,
    ) {
        match msg {
            DkgQueryMsg::GetCurrentEpochState {} => client.get_current_epoch().ignore(),
            DkgQueryMsg::GetCurrentEpochThreshold {} => {
                client.get_current_epoch_threshold().ignore()
            }
            DkgQueryMsg::GetInitialDealers {} => client.get_initial_dealers().ignore(),
            DkgQueryMsg::GetDealerDetails { dealer_address } => client
                .get_dealer_details(&dealer_address.parse().unwrap())
                .ignore(),
            DkgQueryMsg::GetCurrentDealers { limit, start_after } => client
                .get_current_dealers_paged(start_after, limit)
                .ignore(),
            DkgQueryMsg::GetPastDealers { limit, start_after } => {
                client.get_past_dealers_paged(start_after, limit).ignore()
            }
            DkgQueryMsg::GetDealing {
                idx,
                limit,
                start_after,
            } => client.get_dealings_paged(idx, start_after, limit).ignore(),
            DkgQueryMsg::GetVerificationKeys {
                epoch_id,
                limit,
                start_after,
            } => client
                .get_vk_shares_paged(epoch_id, start_after, limit)
                .ignore(),
        };
    }
}
