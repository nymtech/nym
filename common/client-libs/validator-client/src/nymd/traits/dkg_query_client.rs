// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::error::NymdError;
use crate::nymd::{CosmWasmClient, NymdClient};
use async_trait::async_trait;
use coconut_dkg_common::dealer::{
    DealerDetailsResponse, PagedDealerResponse, PagedDealingsResponse,
};
use coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use coconut_dkg_common::types::{EpochState, MinimumDepositResponse};
use cosmrs::AccountId;

#[async_trait]
pub trait DkgQueryClient {
    async fn get_current_epoch_state(&self) -> Result<EpochState, NymdError>;
    async fn get_dealer_details(
        &self,
        address: &AccountId,
    ) -> Result<DealerDetailsResponse, NymdError>;
    async fn get_current_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NymdError>;
    async fn get_past_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NymdError>;

    async fn get_deposit_amount(&self) -> Result<MinimumDepositResponse, NymdError>;
    async fn get_dealings_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealingsResponse, NymdError>;
}

#[async_trait]
impl<C> DkgQueryClient for NymdClient<C>
where
    C: CosmWasmClient + Send + Sync,
{
    async fn get_current_epoch_state(&self) -> Result<EpochState, NymdError> {
        let request = DkgQueryMsg::GetCurrentEpochState {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }
    async fn get_dealer_details(
        &self,
        address: &AccountId,
    ) -> Result<DealerDetailsResponse, NymdError> {
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
    ) -> Result<PagedDealerResponse, NymdError> {
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
    ) -> Result<PagedDealerResponse, NymdError> {
        let request = DkgQueryMsg::GetPastDealers {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }

    async fn get_deposit_amount(&self) -> Result<MinimumDepositResponse, NymdError> {
        let request = DkgQueryMsg::GetDepositAmount {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }

    async fn get_dealings_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealingsResponse, NymdError> {
        let request = DkgQueryMsg::GetDealings {
            limit: page_limit,
            start_after,
        };
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address(), &request)
            .await
    }
}
