use super::VestingPeriod;
use crate::errors::ContractError;
use crate::storage::{
    load_balance, load_bond_pledge, load_gateway_pledge, remove_bond_pledge, remove_delegation,
    remove_gateway_pledge, save_account, save_balance, save_bond_pledge, save_gateway_pledge,
    DELEGATIONS, KEY,
};
use cosmwasm_std::{Addr, Coin, Order, Storage, Timestamp, Uint128};
use cw_storage_plus::Bound;
use mixnet_contract_common::IdentityKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vesting_contract_common::{Period, PledgeData};

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

    pub fn num_vesting_periods(&self) -> usize {
        self.periods.len()
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
        if amount < self.num_vesting_periods() as u128 {
            Err(ContractError::ImprobableVestingAmount(amount))
        } else {
            // Remainder tokens will be lumped into the last period.
            Ok(amount / self.num_vesting_periods() as u128)
        }
    }

    pub fn get_current_vesting_period(&self, block_time: Timestamp) -> Period {
        // Returns the index of the next vesting period. Unless the current time is somehow in the past or vesting has not started yet.
        // In case vesting is over it will always return NUM_VESTING_PERIODS.

        if block_time.seconds() < self.periods.first().unwrap().start_time {
            Period::Before
        } else if self.periods.last().unwrap().end_time() < block_time {
            Period::After
        } else {
            let mut index = 0;
            for period in &self.periods {
                if block_time < period.end_time() {
                    break;
                }
                index += 1;
            }
            Period::In(index)
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

    pub fn any_delegation_for_mix(&self, mix: &str, storage: &dyn Storage) -> bool {
        DELEGATIONS
            .prefix((self.storage_key(), mix.to_string()))
            .range(storage, None, None, Order::Ascending)
            .next()
            .is_some()
    }

    pub fn remove_delegations_for_mix(
        &self,
        mix: &str,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let limit = 50;
        let mut start_after = None;
        let mut block_heights = Vec::new();
        let mut prev_len = 0;
        // TODO: Test this
        loop {
            block_heights.extend(
                DELEGATIONS
                    .prefix((self.storage_key(), mix.to_string()))
                    .keys(storage, start_after, None, Order::Ascending)
                    .take(limit)
                    .filter_map(|key| key.ok()),
            );

            if prev_len == block_heights.len() {
                break;
            }

            prev_len = block_heights.len();

            start_after = block_heights.last().map(|last| Bound::exclusive_int(*last));
            if start_after.is_none() {
                break;
            }
        }

        for block_height in block_heights {
            remove_delegation((self.storage_key(), mix.to_string(), block_height), storage)?;
        }
        Ok(())
    }

    pub fn total_delegations_for_mix(
        &self,
        mix: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Uint128, ContractError> {
        Ok(DELEGATIONS
            .prefix((self.storage_key(), mix))
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .fold(Uint128::zero(), |acc, (_key, val)| acc + val))
    }

    pub fn total_delegations(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        Ok(DELEGATIONS
            .sub_prefix(self.storage_key())
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .fold(Uint128::zero(), |acc, (_key, val)| acc + val))
    }
}
