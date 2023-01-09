// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
pub use crate::nyxd::cosmwasm_client::client::CosmWasmClient;
use crate::nyxd::error::NyxdError;
use crate::nyxd::NyxdClient;
use async_trait::async_trait;
use contracts_common::ContractBuildInformation;
use cosmwasm_std::{Coin as CosmWasmCoin, Timestamp};
use mixnet_contract_common::MixId;
use serde::Deserialize;
use vesting_contract::vesting::Account;
use vesting_contract_common::{
    messages::QueryMsg as VestingQueryMsg, AccountsResponse, AllDelegationsResponse,
    DelegationTimesResponse, OriginalVestingResponse, Period, PledgeData, VestingCoinsResponse,
    VestingDelegation,
};

#[async_trait]
pub trait VestingQueryClient {
    async fn query_vesting_contract<T>(&self, query: VestingQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_vesting_contract_version(&self) -> Result<ContractBuildInformation, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetContractVersion {})
            .await
    }

    async fn get_all_accounts_paged(
        &self,
        start_next_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<AccountsResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetAccountsPaged {
            start_next_after,
            limit,
        })
        .await
    }

    async fn get_all_accounts_locked_coins_paged(
        &self,
        start_next_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<VestingCoinsResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetAccountsLockedCoinsPaged {
            start_next_after,
            limit,
        })
        .await
    }

    async fn locked_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::LockedCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        })
        .await
        .map(Into::into)
    }

    async fn spendable_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::SpendableCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        })
        .await
        .map(Into::into)
    }

    async fn vested_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetVestedCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        })
        .await
        .map(Into::into)
    }

    async fn vesting_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetVestingCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        })
        .await
        .map(Into::into)
    }

    async fn vesting_start_time(
        &self,
        vesting_account_address: &str,
    ) -> Result<Timestamp, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetStartTime {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
    }

    async fn vesting_end_time(
        &self,
        vesting_account_address: &str,
    ) -> Result<Timestamp, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetEndTime {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
    }

    async fn original_vesting(
        &self,
        vesting_account_address: &str,
    ) -> Result<OriginalVestingResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetOriginalVesting {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
    }

    async fn delegated_free(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetDelegatedFree {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        })
        .await
        .map(Into::into)
    }

    /// Returns the total amount of delegated tokens that have vested
    async fn delegated_vesting(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetDelegatedVesting {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        })
        .await
        .map(Into::into)
    }

    async fn get_account(&self, address: &str) -> Result<Account, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetAccount {
            address: address.to_string(),
        })
        .await
    }

    async fn get_mixnode_pledge(&self, address: &str) -> Result<Option<PledgeData>, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetMixnode {
            address: address.to_string(),
        })
        .await
    }

    async fn get_gateway_pledge(&self, address: &str) -> Result<Option<PledgeData>, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetGateway {
            address: address.to_string(),
        })
        .await
    }

    async fn get_current_vesting_period(&self, address: &str) -> Result<Period, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetCurrentVestingPeriod {
            address: address.to_string(),
        })
        .await
    }

    async fn get_delegation_timestamps(
        &self,
        address: &str,
        mix_id: MixId,
    ) -> Result<DelegationTimesResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetDelegationTimes {
            address: address.to_string(),
            mix_id,
        })
        .await
    }

    async fn get_all_vesting_delegations_paged(
        &self,
        start_after: Option<(u32, MixId, u64)>,
        limit: Option<u32>,
    ) -> Result<AllDelegationsResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetAllDelegations { start_after, limit })
            .await
    }

    async fn get_all_vesting_delegations(&self) -> Result<Vec<VestingDelegation>, NyxdError> {
        let mut delegations = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .get_all_vesting_delegations_paged(start_after.take(), None)
                .await?;
            delegations.append(&mut paged_response.delegations);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(delegations)
    }
}

#[async_trait]
impl<C: CosmWasmClient + Sync + Send> VestingQueryClient for NyxdClient<C> {
    async fn query_vesting_contract<T>(&self, query: VestingQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.client
            .query_contract_smart(self.vesting_contract_address(), &query)
            .await
    }
}
