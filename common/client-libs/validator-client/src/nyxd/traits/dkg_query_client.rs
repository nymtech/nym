// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_coconut_dkg_common::dealer::{
    DealerDetailsResponse, PagedDealerResponse, PagedDealingsResponse,
};
use nym_coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use nym_coconut_dkg_common::types::{Epoch, EpochId, InitialReplacementData};
use nym_coconut_dkg_common::verification_key::PagedVKSharesResponse;

#[async_trait]
pub trait DkgQueryClient {
    async fn get_current_epoch(&self) -> Result<Epoch, NyxdError>;
    async fn get_current_epoch_threshold(&self) -> Result<Option<u64>, NyxdError>;
    async fn get_initial_dealers(&self) -> Result<Option<InitialReplacementData>, NyxdError>;
    async fn get_dealer_details(
        &self,
        address: &AccountId,
    ) -> Result<DealerDetailsResponse, NyxdError>;
    async fn get_current_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NyxdError>;
    async fn get_past_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NyxdError>;

    async fn get_dealings_paged(
        &self,
        idx: usize,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealingsResponse, NyxdError>;
    async fn get_vk_shares_paged(
        &self,
        epoch_id: EpochId,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedVKSharesResponse, NyxdError>;
}

#[async_trait]
impl<C> DkgQueryClient for NyxdClient<C>
where
    C: CosmWasmClient + Send + Sync + Clone,
{
    async fn get_current_epoch(&self) -> Result<Epoch, NyxdError> {
        let request = DkgQueryMsg::GetCurrentEpochState {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }
    async fn get_current_epoch_threshold(&self) -> Result<Option<u64>, NyxdError> {
        let request = DkgQueryMsg::GetCurrentEpochThreshold {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }

    async fn get_initial_dealers(&self) -> Result<Option<InitialReplacementData>, NyxdError> {
        let request = DkgQueryMsg::GetInitialDealers {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }

    async fn get_dealer_details(
        &self,
        address: &AccountId,
    ) -> Result<DealerDetailsResponse, NyxdError> {
        let request = DkgQueryMsg::GetDealerDetails {
            dealer_address: address.to_string(),
        };
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
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
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
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
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
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
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
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
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }
}
