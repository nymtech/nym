// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::coin::Coin;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cosmwasm_std::{Coin as CosmWasmCoin, Timestamp};
use nym_contracts_common::ContractBuildInformation;
use nym_mixnet_contract_common::NodeId;
use nym_vesting_contract_common::{
    messages::QueryMsg as VestingQueryMsg, Account, AccountVestingCoins, AccountsResponse,
    AllDelegationsResponse, BaseVestingAccountInfo, DelegationTimesResponse,
    OriginalVestingResponse, Period, PledgeData, VestingCoinsResponse, VestingDelegation,
};
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait VestingQueryClient {
    async fn query_vesting_contract<T>(&self, query: VestingQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_vesting_contract_version(&self) -> Result<ContractBuildInformation, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetContractVersion {})
            .await
    }

    async fn get_vesting_contract_cw2_version(&self) -> Result<cw2::ContractVersion, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetCW2ContractVersion {})
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

    async fn get_all_accounts_vesting_coins_paged(
        &self,
        start_next_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<VestingCoinsResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetAccountsVestingCoinsPaged {
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

    async fn get_historical_vesting_staking_reward(
        &self,
        vesting_account_address: &str,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(
            VestingQueryMsg::GetHistoricalVestingStakingReward {
                vesting_account_address: vesting_account_address.to_string(),
            },
        )
        .await
        .map(Into::into)
    }

    async fn get_spendable_vested_coins(
        &self,
        vesting_account_address: &str,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetSpendableVestedCoins {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
        .map(Into::into)
    }

    async fn get_spendable_reward_coins(
        &self,
        vesting_account_address: &str,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetSpendableRewardCoins {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
        .map(Into::into)
    }

    async fn get_delegated_coins(&self, vesting_account_address: &str) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetDelegatedCoins {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
        .map(Into::into)
    }

    async fn get_pledged_coins(&self, vesting_account_address: &str) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetPledgedCoins {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
        .map(Into::into)
    }

    async fn get_staked_coins(&self, vesting_account_address: &str) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetStakedCoins {
            vesting_account_address: vesting_account_address.to_string(),
        })
        .await
        .map(Into::into)
    }

    async fn get_withdrawn_coins(&self, vesting_account_address: &str) -> Result<Coin, NyxdError> {
        self.query_vesting_contract::<CosmWasmCoin>(VestingQueryMsg::GetWithdrawnCoins {
            vesting_account_address: vesting_account_address.to_string(),
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

    async fn get_vesting_delegation(
        &self,
        address: &str,
        mix_id: NodeId,
        block_timestamp_secs: u64,
    ) -> Result<VestingDelegation, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetDelegation {
            address: address.to_string(),
            mix_id,
            block_timestamp_secs,
        })
        .await
    }

    async fn get_total_delegation_amount(
        &self,
        address: &str,
        mix_id: NodeId,
    ) -> Result<Coin, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetTotalDelegationAmount {
            address: address.to_string(),
            mix_id,
        })
        .await
    }

    async fn get_delegation_timestamps(
        &self,
        address: &str,
        mix_id: NodeId,
    ) -> Result<DelegationTimesResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetDelegationTimes {
            address: address.to_string(),
            mix_id,
        })
        .await
    }

    async fn get_all_vesting_delegations_paged(
        &self,
        start_after: Option<(u32, NodeId, u64)>,
        limit: Option<u32>,
    ) -> Result<AllDelegationsResponse, NyxdError> {
        self.query_vesting_contract(VestingQueryMsg::GetAllDelegations { start_after, limit })
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedVestingQueryClient: VestingQueryClient {
    async fn get_all_vesting_delegations(&self) -> Result<Vec<VestingDelegation>, NyxdError> {
        collect_paged!(self, get_all_vesting_delegations_paged, delegations)
    }

    async fn get_all_accounts_info(&self) -> Result<Vec<BaseVestingAccountInfo>, NyxdError> {
        collect_paged!(self, get_all_accounts_paged, accounts)
    }

    async fn get_all_accounts_vesting_coins(&self) -> Result<Vec<AccountVestingCoins>, NyxdError> {
        collect_paged!(self, get_all_accounts_vesting_coins_paged, accounts)
    }
}

#[async_trait]
impl<T> PagedVestingQueryClient for T where T: VestingQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> VestingQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_vesting_contract<T>(&self, query: VestingQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let vesting_contract_address = &self
            .vesting_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("vesting contract"))?;
        self.query_contract_smart(vesting_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: VestingQueryClient + Send + Sync>(
        client: C,
        msg: VestingQueryMsg,
    ) {
        match msg {
            VestingQueryMsg::GetContractVersion {} => {
                client.get_vesting_contract_version().ignore()
            }
            VestingQueryMsg::GetCW2ContractVersion {} => {
                client.get_vesting_contract_cw2_version().ignore()
            }
            VestingQueryMsg::GetAccountsPaged {
                start_next_after,
                limit,
            } => client
                .get_all_accounts_paged(start_next_after, limit)
                .ignore(),
            VestingQueryMsg::GetAccountsVestingCoinsPaged {
                start_next_after,
                limit,
            } => client
                .get_all_accounts_vesting_coins_paged(start_next_after, limit)
                .ignore(),
            VestingQueryMsg::LockedCoins {
                vesting_account_address,
                block_time,
            } => client
                .locked_coins(&vesting_account_address, block_time)
                .ignore(),
            VestingQueryMsg::SpendableCoins {
                vesting_account_address,
                block_time,
            } => client
                .spendable_coins(&vesting_account_address, block_time)
                .ignore(),
            VestingQueryMsg::GetVestedCoins {
                vesting_account_address,
                block_time,
            } => client
                .vested_coins(&vesting_account_address, block_time)
                .ignore(),
            VestingQueryMsg::GetVestingCoins {
                vesting_account_address,
                block_time,
            } => client
                .vesting_coins(&vesting_account_address, block_time)
                .ignore(),
            VestingQueryMsg::GetStartTime {
                vesting_account_address,
            } => client.vesting_start_time(&vesting_account_address).ignore(),
            VestingQueryMsg::GetEndTime {
                vesting_account_address,
            } => client.vesting_end_time(&vesting_account_address).ignore(),
            VestingQueryMsg::GetOriginalVesting {
                vesting_account_address,
            } => client.original_vesting(&vesting_account_address).ignore(),
            VestingQueryMsg::GetHistoricalVestingStakingReward {
                vesting_account_address,
            } => client
                .get_historical_vesting_staking_reward(&vesting_account_address)
                .ignore(),
            VestingQueryMsg::GetSpendableVestedCoins {
                vesting_account_address,
            } => client
                .get_spendable_vested_coins(&vesting_account_address)
                .ignore(),
            VestingQueryMsg::GetSpendableRewardCoins {
                vesting_account_address,
            } => client
                .get_spendable_reward_coins(&vesting_account_address)
                .ignore(),
            VestingQueryMsg::GetDelegatedCoins {
                vesting_account_address,
            } => client
                .get_delegated_coins(&vesting_account_address)
                .ignore(),
            VestingQueryMsg::GetPledgedCoins {
                vesting_account_address,
            } => client.get_pledged_coins(&vesting_account_address).ignore(),
            VestingQueryMsg::GetStakedCoins {
                vesting_account_address,
            } => client.get_staked_coins(&vesting_account_address).ignore(),
            VestingQueryMsg::GetWithdrawnCoins {
                vesting_account_address,
            } => client
                .get_withdrawn_coins(&vesting_account_address)
                .ignore(),
            VestingQueryMsg::GetAccount { address } => client.get_account(&address).ignore(),
            VestingQueryMsg::GetMixnode { address } => client.get_mixnode_pledge(&address).ignore(),
            VestingQueryMsg::GetGateway { address } => client.get_gateway_pledge(&address).ignore(),
            VestingQueryMsg::GetCurrentVestingPeriod { address } => {
                client.get_current_vesting_period(&address).ignore()
            }
            VestingQueryMsg::GetDelegation {
                address,
                mix_id,
                block_timestamp_secs,
            } => client
                .get_vesting_delegation(&address, mix_id, block_timestamp_secs)
                .ignore(),
            VestingQueryMsg::GetTotalDelegationAmount { address, mix_id } => client
                .get_total_delegation_amount(&address, mix_id)
                .ignore(),
            VestingQueryMsg::GetDelegationTimes { address, mix_id } => {
                client.get_delegation_timestamps(&address, mix_id).ignore()
            }
            VestingQueryMsg::GetAllDelegations { start_after, limit } => client
                .get_all_vesting_delegations_paged(start_after, limit)
                .ignore(),
        };
    }
}
