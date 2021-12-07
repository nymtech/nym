// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::error::NymdError;
use crate::nymd::NymdClient;
use async_trait::async_trait;
use cosmwasm_std::{Coin, Timestamp};
use vesting_contract::messages::QueryMsg as VestingQueryMsg;

#[async_trait]
pub trait VestingQueryClient {
    async fn locked_coins(
        &self,
        address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError>;

    async fn spendable_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError>;

    async fn vested_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError>;

    async fn vesting_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError>;

    async fn vesting_start_time(
        &self,
        vesting_account_address: &str,
    ) -> Result<Timestamp, NymdError>;

    async fn vesting_end_time(&self, vesting_account_address: &str)
        -> Result<Timestamp, NymdError>;

    async fn original_vesting(&self, vesting_account_address: &str) -> Result<Coin, NymdError>;

    async fn delegated_free(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError>;

    async fn delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        vesting_account_address: &str,
    ) -> Result<Coin, NymdError>;
}

#[async_trait]
impl<C: CosmWasmClient + Sync + Send> VestingQueryClient for NymdClient<C> {
    async fn locked_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError> {
        let request = VestingQueryMsg::LockedCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }

    async fn spendable_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError> {
        let request = VestingQueryMsg::SpendableCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }
    async fn vested_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError> {
        let request = VestingQueryMsg::GetVestedCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }
    async fn vesting_coins(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError> {
        let request = VestingQueryMsg::GetVestingCoins {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }

    async fn vesting_start_time(
        &self,
        vesting_account_address: &str,
    ) -> Result<Timestamp, NymdError> {
        let request = VestingQueryMsg::GetStartTime {
            vesting_account_address: vesting_account_address.to_string(),
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }

    async fn vesting_end_time(
        &self,
        vesting_account_address: &str,
    ) -> Result<Timestamp, NymdError> {
        let request = VestingQueryMsg::GetEndTime {
            vesting_account_address: vesting_account_address.to_string(),
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }

    async fn original_vesting(&self, vesting_account_address: &str) -> Result<Coin, NymdError> {
        let request = VestingQueryMsg::GetOriginalVesting {
            vesting_account_address: vesting_account_address.to_string(),
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }

    async fn delegated_free(
        &self,
        vesting_account_address: &str,
        block_time: Option<Timestamp>,
    ) -> Result<Coin, NymdError> {
        let request = VestingQueryMsg::GetDelegatedFree {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }

    async fn delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        vesting_account_address: &str,
    ) -> Result<Coin, NymdError> {
        let request = VestingQueryMsg::GetDelegatedVesting {
            vesting_account_address: vesting_account_address.to_string(),
            block_time,
        };
        self.client
            .query_contract_smart(self.vesting_contract_address()?, &request)
            .await
    }
}
