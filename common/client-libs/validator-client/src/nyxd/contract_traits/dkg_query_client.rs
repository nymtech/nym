// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_coconut_dkg_common::dealer::{
    ContractDealing, DealerDetailsResponse, PagedDealerResponse, PagedDealingsResponse,
};
use nym_coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use nym_coconut_dkg_common::types::{DealerDetails, Epoch, EpochId, InitialReplacementData};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, PagedVKSharesResponse};
use serde::Deserialize;

#[async_trait]
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
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NyxdError> {
        let request = DkgQueryMsg::GetCurrentDealers {
            start_after,
            limit: page_limit,
        };
        self.query_dkg_contract(request).await
    }

    async fn get_past_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NyxdError> {
        let request = DkgQueryMsg::GetPastDealers {
            start_after,
            limit: page_limit,
        };
        self.query_dkg_contract(request).await
    }

    async fn get_dealings_paged(
        &self,
        idx: usize,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealingsResponse, NyxdError> {
        let request = DkgQueryMsg::GetDealing {
            idx: idx as u64,
            limit: page_limit,
            start_after,
        };
        self.query_dkg_contract(request).await
    }

    async fn get_vk_shares_paged(
        &self,
        epoch_id: EpochId,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedVKSharesResponse, NyxdError> {
        let request = DkgQueryMsg::GetVerificationKeys {
            epoch_id,
            limit: page_limit,
            start_after,
        };
        self.query_dkg_contract(request).await
    }

    async fn get_all_current_dealers(&self) -> Result<Vec<DealerDetails>, NyxdError> {
        let mut dealers = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .get_current_dealers_paged(start_after.take(), None)
                .await?;
            dealers.append(&mut paged_response.dealers);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(dealers)
    }

    async fn get_all_past_dealers(&self) -> Result<Vec<DealerDetails>, NyxdError> {
        let mut dealers = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .get_past_dealers_paged(start_after.take(), None)
                .await?;
            dealers.append(&mut paged_response.dealers);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(dealers)
    }

    async fn get_all_epoch_dealings(&self, idx: usize) -> Result<Vec<ContractDealing>, NyxdError> {
        let mut dealings = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .get_dealings_paged(idx, start_after.take(), None)
                .await?;
            dealings.append(&mut paged_response.dealings);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(dealings)
    }

    async fn get_all_verification_key_shares(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<ContractVKShare>, NyxdError> {
        let mut shares = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .get_vk_shares_paged(epoch_id, start_after.take(), None)
                .await?;
            shares.append(&mut paged_response.shares);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(shares)
    }
}

#[async_trait]
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

    // it's enough that this compiles
    #[deprecated]
    async fn all_query_variants_are_covered<C: DkgQueryClient + Send + Sync>(
        client: C,
        msg: DkgQueryMsg,
    ) {
        unimplemented!()
    }
}
