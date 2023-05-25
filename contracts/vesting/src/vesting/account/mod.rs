use super::VestingPeriod;
use crate::errors::ContractError;
use crate::storage::{
    count_subdelegations_for_mix, decrease_bond_pledge, load_balance, load_bond_pledge,
    load_delegation_timestamps, load_gateway_pledge, load_withdrawn, remove_bond_pledge,
    remove_delegation, remove_gateway_pledge, save_account, save_balance, save_bond_pledge,
    save_gateway_pledge, save_withdrawn, AccountStorageKey, BlockTimestampSecs, DELEGATIONS, KEY,
};
use crate::traits::VestingAccount;
use cosmwasm_std::{Addr, Coin, Order, Storage, Timestamp, Uint128};
use mixnet_contract_common::MixId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vesting_contract_common::{Period, PledgeCap, PledgeData};

mod delegating_account;
mod gateway_bonding_account;
mod mixnode_bonding_account;
mod node_families;
mod vesting_account;

fn generate_storage_key(storage: &mut dyn Storage) -> Result<AccountStorageKey, ContractError> {
    let key = KEY.may_load(storage)?.unwrap_or(0) + 1;
    KEY.save(storage, &key)?;
    Ok(key)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Account {
    pub owner_address: Addr,
    pub staking_address: Option<Addr>,
    pub start_time: Timestamp,
    pub periods: Vec<VestingPeriod>,
    pub coin: Coin,
    storage_key: AccountStorageKey,
    #[serde(default)]
    pub pledge_cap: Option<PledgeCap>,
}

impl Account {
    pub fn new(
        owner_address: Addr,
        staking_address: Option<Addr>,
        coin: Coin,
        start_time: Timestamp,
        periods: Vec<VestingPeriod>,
        pledge_cap: Option<PledgeCap>,
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
            pledge_cap,
        };
        save_account(&account, storage)?;
        account.save_balance(amount, storage)?;
        Ok(account)
    }

    /// Checks whether the additional stake would be within the cap associated with the account
    /// and whether the account has enough tokens for staking.
    /// Returns the value of the current balance of the account.
    pub fn ensure_valid_additional_stake(
        &self,
        additional_stake: &Coin,
        storage: &dyn Storage,
    ) -> Result<Uint128, ContractError> {
        let current_balance = self.load_balance(storage)?;
        let current_total_staked = self.total_staked(storage)?;
        let total_staked_after = current_total_staked + additional_stake.amount;
        let locked_pledge_cap = self.absolute_pledge_cap()?;

        if locked_pledge_cap < total_staked_after {
            return Err(ContractError::LockedPledgeCapReached {
                current: total_staked_after,
                cap: locked_pledge_cap,
            });
        }

        if current_balance < additional_stake.amount {
            return Err(ContractError::InsufficientBalance(
                self.owner_address().as_str().to_string(),
                current_balance.u128(),
            ));
        }

        Ok(current_balance)
    }

    pub fn pledge_cap(&self) -> PledgeCap {
        self.pledge_cap.clone().unwrap_or_default()
    }

    pub fn absolute_pledge_cap(&self) -> Result<Uint128, ContractError> {
        match self.pledge_cap() {
            PledgeCap::Absolute(cap) => Ok(cap),
            PledgeCap::Percent(p) => Ok(p * self.get_original_vesting()?.amount.amount),
        }
    }

    pub fn coin(&self) -> Coin {
        self.coin.clone()
    }

    pub fn num_vesting_periods(&self) -> usize {
        self.periods.len()
    }

    pub fn period_duration(&self) -> Result<u64, ContractError> {
        self.periods
            .get(0)
            .ok_or(ContractError::UnpopulatedVestingPeriods {
                owner: self.owner_address.clone(),
            })
            .map(|p| p.period_seconds)
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

    /// Returns the index of the next vesting period. Unless the current time is somehow in the past or vesting has not started yet.
    /// In case vesting is over it will always return NUM_VESTING_PERIODS.
    pub fn get_current_vesting_period(
        &self,
        block_time: Timestamp,
    ) -> Result<Period, ContractError> {
        let first_period =
            self.periods
                .first()
                .ok_or(ContractError::UnpopulatedVestingPeriods {
                    owner: self.owner_address.clone(),
                })?;

        let last_period = self
            .periods
            .last()
            .ok_or(ContractError::UnpopulatedVestingPeriods {
                owner: self.owner_address.clone(),
            })?;

        if block_time.seconds() < first_period.start_time {
            Ok(Period::Before)
        } else if last_period.end_time() < block_time {
            Ok(Period::After)
        } else {
            let mut index = 0;
            for period in &self.periods {
                if block_time < period.end_time() {
                    break;
                }
                index += 1;
            }
            Ok(Period::In(index))
        }
    }

    pub fn withdraw(
        &self,
        amount: &Coin,
        storage: &mut dyn Storage,
    ) -> Result<u128, ContractError> {
        let new_balance = self
            .load_balance(storage)?
            .u128()
            .saturating_sub(amount.amount.u128());
        self.save_balance(Uint128::new(new_balance), storage)?;
        let withdrawn = self.load_withdrawn(storage)?;
        self.save_withdrawn(withdrawn + amount.amount, storage)?;
        Ok(new_balance)
    }

    pub fn load_withdrawn(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        load_withdrawn(self.storage_key, storage)
    }

    pub fn save_withdrawn(
        &self,
        withdrawn: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        save_withdrawn(self.storage_key, withdrawn, storage)
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

    pub fn decrease_mixnode_pledge(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        decrease_bond_pledge(self.storage_key, amount, storage)
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

    pub fn any_delegation_for_mix(&self, mix_id: MixId, storage: &dyn Storage) -> bool {
        DELEGATIONS
            .prefix((self.storage_key(), mix_id))
            .range(storage, None, None, Order::Ascending)
            .next()
            .is_some()
    }

    pub fn num_subdelegations_for_mix(&self, mix_id: MixId, storage: &dyn Storage) -> u32 {
        count_subdelegations_for_mix((self.storage_key(), mix_id), storage)
    }

    pub fn remove_delegations_for_mix(
        &self,
        mix_id: MixId,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        // note that the limit is implicitly set to `MAX_PER_MIX_DELEGATIONS`
        // as it should be impossible to create more delegations than that.
        let block_timestamps = load_delegation_timestamps((self.storage_key(), mix_id), storage)?;

        for block_timestamp in block_timestamps {
            remove_delegation((self.storage_key(), mix_id, block_timestamp), storage)?;
        }
        Ok(())
    }

    pub fn total_delegations_for_mix(
        &self,
        mix_id: MixId,
        storage: &dyn Storage,
    ) -> Result<Uint128, ContractError> {
        Ok(DELEGATIONS
            .prefix((self.storage_key(), mix_id))
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .fold(Uint128::zero(), |acc, (_key, val)| acc + val))
    }

    // TODO: this should get reworked... somehow... (maybe with a memoized value?)
    // as it's an unbounded iteration that could fail if an account has made a lot of delegations
    // (I guess in order of thousands)
    pub fn total_delegations(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        Ok(DELEGATIONS
            .sub_prefix(self.storage_key())
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .fold(Uint128::zero(), |acc, (_key, val)| acc + val))
    }

    pub fn total_pledged(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        let amount = if let Some(bond) = self
            .load_mixnode_pledge(storage)?
            .or(self.load_gateway_pledge(storage)?)
        {
            bond.amount().amount
        } else {
            Uint128::zero()
        };
        Ok(amount)
    }

    pub fn total_delegations_at_timestamp(
        &self,
        storage: &dyn Storage,
        start_time: BlockTimestampSecs,
    ) -> Result<Uint128, ContractError> {
        Ok(DELEGATIONS
            .sub_prefix(self.storage_key())
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .filter(|((_mix, block_time), _amount)| *block_time <= start_time)
            .fold(Uint128::zero(), |acc, ((_mix, _block_time), amount)| {
                acc + amount
            }))
    }
}
