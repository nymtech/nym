// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::VestingPeriod;
use crate::storage::{
    count_subdelegations_for_mix, decrease_bond_pledge, load_balance, load_bond_pledge,
    load_delegation_timestamps, load_gateway_pledge, load_withdrawn, remove_bond_pledge,
    remove_delegation, remove_gateway_pledge, save_account, save_balance, save_bond_pledge,
    save_gateway_pledge, save_withdrawn, BlockTimestampSecs, DELEGATIONS, KEY,
};
use crate::traits::VestingAccount;
use cosmwasm_std::{Addr, Coin, Order, Storage, Timestamp, Uint128};
use mixnet_contract_common::NodeId;
use vesting_contract_common::account::VestingAccountStorageKey;
use vesting_contract_common::{Account, PledgeCap, PledgeData, VestingContractError};

mod delegating_account;
mod gateway_bonding_account;
mod mixnode_bonding_account;
mod vesting_account;

fn generate_storage_key(
    storage: &mut dyn Storage,
) -> Result<VestingAccountStorageKey, VestingContractError> {
    let key = KEY.may_load(storage)?.unwrap_or(0) + 1;
    KEY.save(storage, &key)?;
    Ok(key)
}

/// Helper trait to extend the `Account` type with methods that require access to the underlying storage
pub(crate) trait StorableVestingAccountExt: VestingAccount {
    fn save_new(
        owner_address: Addr,
        staking_address: Option<Addr>,
        coin: Coin,
        start_time: Timestamp,
        periods: Vec<VestingPeriod>,
        pledge_cap: Option<PledgeCap>,
        storage: &mut dyn Storage,
    ) -> Result<Account, VestingContractError> {
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

    fn owner_address(&self) -> Addr;

    /// Checks whether the additional stake would be within the cap associated with the account
    /// and whether the account has enough tokens for staking.
    /// Returns the value of the current balance of the account.
    fn ensure_valid_additional_stake(
        &self,
        additional_stake: &Coin,
        storage: &dyn Storage,
    ) -> Result<Uint128, VestingContractError> {
        let current_balance = self.load_balance(storage)?;
        let current_total_staked = self.total_staked(storage)?;
        let total_staked_after = current_total_staked + additional_stake.amount;
        let locked_pledge_cap = self.absolute_pledge_cap()?;

        if locked_pledge_cap < total_staked_after {
            return Err(VestingContractError::LockedPledgeCapReached {
                current: total_staked_after,
                cap: locked_pledge_cap,
            });
        }

        if current_balance < additional_stake.amount {
            return Err(VestingContractError::InsufficientBalance(
                self.owner_address().into_string(),
                current_balance.u128(),
            ));
        }

        Ok(current_balance)
    }

    fn absolute_pledge_cap(&self) -> Result<Uint128, VestingContractError>;

    fn withdraw(
        &self,
        amount: &Coin,
        storage: &mut dyn Storage,
    ) -> Result<u128, VestingContractError> {
        let new_balance = self
            .load_balance(storage)?
            .u128()
            .saturating_sub(amount.amount.u128());
        self.save_balance(Uint128::new(new_balance), storage)?;
        let withdrawn = self.load_withdrawn(storage)?;
        self.save_withdrawn(withdrawn + amount.amount, storage)?;
        Ok(new_balance)
    }

    fn load_withdrawn(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError>;

    fn save_withdrawn(
        &self,
        withdrawn: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;

    fn load_balance(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError>;

    fn save_balance(
        &self,
        amount: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;

    fn load_mixnode_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, VestingContractError>;

    fn save_mixnode_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;

    fn remove_mixnode_pledge(&self, storage: &mut dyn Storage) -> Result<(), VestingContractError>;

    fn decrease_mixnode_pledge(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;

    fn load_gateway_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, VestingContractError>;

    fn save_gateway_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;

    fn remove_gateway_pledge(&self, storage: &mut dyn Storage) -> Result<(), VestingContractError>;

    fn any_delegation_for_mix(&self, mix_id: NodeId, storage: &dyn Storage) -> bool;

    fn num_subdelegations_for_mix(&self, mix_id: NodeId, storage: &dyn Storage) -> u32;

    fn remove_delegations_for_mix(
        &self,
        mix_id: NodeId,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;

    #[allow(dead_code)]
    fn total_delegations_for_mix(
        &self,
        mix_id: NodeId,
        storage: &dyn Storage,
    ) -> Result<Uint128, VestingContractError>;

    fn total_delegations(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError>;

    fn total_pledged(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError> {
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

    #[allow(dead_code)]
    fn total_delegations_at_timestamp(
        &self,
        storage: &dyn Storage,
        start_time: BlockTimestampSecs,
    ) -> Result<Uint128, VestingContractError>;
}

impl StorableVestingAccountExt for Account {
    fn owner_address(&self) -> Addr {
        self.owner_address.clone()
    }

    fn absolute_pledge_cap(&self) -> Result<Uint128, VestingContractError> {
        match self.pledge_cap() {
            PledgeCap::Absolute(cap) => Ok(cap),
            PledgeCap::Percent(p) => Ok(p * self.get_original_vesting()?.amount.amount),
        }
    }

    fn load_withdrawn(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError> {
        load_withdrawn(self.storage_key, storage)
    }

    fn save_withdrawn(
        &self,
        withdrawn: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        save_withdrawn(self.storage_key, withdrawn, storage)
    }

    fn load_balance(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError> {
        load_balance(self.storage_key(), storage)
    }

    fn save_balance(
        &self,
        amount: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        save_balance(self.storage_key(), amount, storage)
    }

    fn load_mixnode_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, VestingContractError> {
        load_bond_pledge(self.storage_key(), storage)
    }

    fn save_mixnode_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        save_bond_pledge(self.storage_key(), &pledge, storage)
    }

    fn remove_mixnode_pledge(&self, storage: &mut dyn Storage) -> Result<(), VestingContractError> {
        remove_bond_pledge(self.storage_key(), storage)
    }

    fn decrease_mixnode_pledge(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        decrease_bond_pledge(self.storage_key(), amount, storage)
    }

    fn load_gateway_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, VestingContractError> {
        load_gateway_pledge(self.storage_key(), storage)
    }

    fn save_gateway_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        save_gateway_pledge(self.storage_key(), &pledge, storage)
    }

    fn remove_gateway_pledge(&self, storage: &mut dyn Storage) -> Result<(), VestingContractError> {
        remove_gateway_pledge(self.storage_key(), storage)
    }

    fn any_delegation_for_mix(&self, mix_id: NodeId, storage: &dyn Storage) -> bool {
        DELEGATIONS
            .prefix((self.storage_key(), mix_id))
            .range(storage, None, None, Order::Ascending)
            .next()
            .is_some()
    }

    fn num_subdelegations_for_mix(&self, mix_id: NodeId, storage: &dyn Storage) -> u32 {
        count_subdelegations_for_mix((self.storage_key(), mix_id), storage)
    }

    fn remove_delegations_for_mix(
        &self,
        mix_id: NodeId,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        // note that the limit is implicitly set to `MAX_PER_MIX_DELEGATIONS`
        // as it should be impossible to create more delegations than that.
        let block_timestamps = load_delegation_timestamps((self.storage_key(), mix_id), storage)?;

        for block_timestamp in block_timestamps {
            remove_delegation((self.storage_key(), mix_id, block_timestamp), storage)?;
        }
        Ok(())
    }

    fn total_delegations_for_mix(
        &self,
        mix_id: NodeId,
        storage: &dyn Storage,
    ) -> Result<Uint128, VestingContractError> {
        Ok(DELEGATIONS
            .prefix((self.storage_key(), mix_id))
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .fold(Uint128::zero(), |acc, (_key, val)| acc + val))
    }

    fn total_delegations(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError> {
        Ok(DELEGATIONS
            .sub_prefix(self.storage_key())
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .fold(Uint128::zero(), |acc, (_key, val)| acc + val))
    }

    fn total_delegations_at_timestamp(
        &self,
        storage: &dyn Storage,
        start_time: BlockTimestampSecs,
    ) -> Result<Uint128, VestingContractError> {
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
