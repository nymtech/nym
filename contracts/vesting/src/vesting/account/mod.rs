use super::{PledgeData, VestingPeriod};
use crate::contract::NUM_VESTING_PERIODS;
use crate::errors::ContractError;
use crate::storage::{
    load_balance, load_bond_pledge, load_delegations_all, load_delegations_for_mix,
    load_gateway_pledge, remove_bond_pledge, remove_delegation, remove_gateway_pledge,
    save_account, save_balance, save_bond_pledge, save_gateway_pledge, KEY,
};
use cosmwasm_std::{Addr, Coin, Storage, Timestamp, Uint128};
use mixnet_contract_common::IdentityKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod delegating_account;
mod gateway_bonding_account;
mod mixnode_bonding_account;
mod vesting_account;

fn generate_storage_key(storage: &mut dyn Storage) -> Result<u32, ContractError> {
    let key = KEY.may_load(storage)?.unwrap_or(0) + 1;
    KEY.save(storage, &key)?;
    Ok(key)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Account {
    owner_address: Addr,
    staking_address: Option<Addr>,
    start_time: Timestamp,
    periods: Vec<VestingPeriod>,
    coin: Coin,
    storage_key: u32,
}

impl Account {
    pub fn new(
        owner_address: Addr,
        staking_address: Option<Addr>,
        coin: Coin,
        start_time: Timestamp,
        periods: Vec<VestingPeriod>,
        storage: &mut dyn Storage,
    ) -> Result<Self, ContractError> {
        let storage_key = generate_storage_key(storage)?;
        let amount = coin.amount;
        let account = Account {
            owner_address,
            staking_address,
            start_time,
            periods,
            coin,
            storage_key,
        };
        save_account(&account, storage)?;
        account.save_balance(amount, storage)?;
        Ok(account)
    }

    pub fn storage_key(&self) -> u32 {
        self.storage_key
    }

    pub fn owner_address(&self) -> Addr {
        self.owner_address.clone()
    }

    pub fn staking_address(&self) -> Option<&Addr> {
        self.staking_address.as_ref()
    }

    #[allow(dead_code)]
    pub fn periods(&self) -> Vec<VestingPeriod> {
        self.periods.clone()
    }

    #[allow(dead_code)]
    pub fn start_time(&self) -> Timestamp {
        self.start_time
    }

    pub fn tokens_per_period(&self) -> Result<u128, ContractError> {
        let amount = self.coin.amount.u128();
        if amount < NUM_VESTING_PERIODS as u128 {
            Err(ContractError::ImprobableVestingAmount(amount))
        } else {
            // Remainder tokens will be lumped into the last period.
            Ok(amount / NUM_VESTING_PERIODS as u128)
        }
    }

    pub fn get_current_vesting_period(&self, block_time: Timestamp) -> usize {
        // Returns the index of the next vesting period. Unless the current time is somehow in the past or vesting has not started yet.
        // In case vesting is over it will always return NUM_VESTING_PERIODS.
        let period = match self
            .periods
            .iter()
            .map(|period| period.start_time)
            .collect::<Vec<u64>>()
            .binary_search(&block_time.seconds())
        {
            Ok(u) => u,
            Err(u) => u,
        };

        if period > 0 {
            period - 1
        } else {
            0
        }
    }

    pub fn load_balance(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        load_balance(self.storage_key(), storage)
    }

    pub fn save_balance(
        &self,
        amount: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        save_balance(self.storage_key(), amount, storage)
    }

    pub fn load_mixnode_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, ContractError> {
        load_bond_pledge(self.storage_key(), storage)
    }

    pub fn save_mixnode_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        save_bond_pledge(self.storage_key(), &pledge, storage)
    }

    pub fn remove_mixnode_pledge(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        remove_bond_pledge(self.storage_key(), storage)
    }

    pub fn load_gateway_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, ContractError> {
        load_gateway_pledge(self.storage_key(), storage)
    }

    pub fn save_gateway_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        save_gateway_pledge(self.storage_key(), &pledge, storage)
    }

    pub fn remove_gateway_pledge(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        remove_gateway_pledge(self.storage_key(), storage)
    }

    // Returns block_time part of the delegation key
    pub fn delegation_block_times_for_mix(
        &self,
        mix: &str,
        storage: &dyn Storage,
    ) -> Result<Vec<u64>, ContractError> {
        let delegations = load_delegations_for_mix(self.storage_key(), mix, storage)?;
        Ok(delegations
            .into_iter()
            .map(|delegation| delegation.0)
            .collect::<Vec<u64>>())
    }

    pub fn any_delegation_for_mix(
        &self,
        mix: &str,
        storage: &dyn Storage,
    ) -> Result<bool, ContractError> {
        Ok(!self
            .delegation_block_times_for_mix(mix, storage)?
            .is_empty())
    }

    pub fn remove_delegations_for_mix(
        &self,
        mix: IdentityKey,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let delgation_keys = self.delegation_block_times_for_mix(&mix, storage)?;
        for key in delgation_keys {
            remove_delegation((self.storage_key(), mix.as_bytes(), key), storage)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn total_delegations_for_mix(
        &self,
        mix: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Uint128, ContractError> {
        Ok(load_delegations_for_mix(self.storage_key(), &mix, storage)?
            .iter()
            .fold(Uint128::zero(), |acc, (_key, val)| acc + *val))
    }

    pub fn total_delegations(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        let delegations = load_delegations_all(self.storage_key(), storage)?;
        Ok(delegations
            .into_iter()
            .fold(Uint128::zero(), |acc, x| acc + x.1))
    }
}
